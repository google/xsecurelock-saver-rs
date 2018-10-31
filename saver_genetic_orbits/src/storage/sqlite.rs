// Copyright 2018 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate rusqlite;

use std::error::Error;
use std::path::Path;

use self::rusqlite::{
    Connection,
    Error as SqlError,
    types::{
        FromSql,
        FromSqlError,
        ToSql,
        ToSqlOutput,
        Value as SqlValue,
        ValueRef as SqlValueRef,
    },
};
use serde_json;

use storage::Storage;
use model::{Scenario, World};

pub struct SqliteStorage {
    conn: Connection,
}

// This is safe because all methods on SqliteStorage take &mut self, so sharing &self across
// threads is safe (though not useful).
unsafe impl Sync for SqliteStorage {}

impl SqliteStorage {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<SqliteStorage, SqlError> {
        Connection::open(path).and_then(SqliteStorage::from_conn)
    }

    pub fn open_in_memory() -> Result<SqliteStorage, SqlError> {
        Connection::open_in_memory().and_then(SqliteStorage::from_conn)
    }

    fn from_conn(conn: Connection) -> Result<SqliteStorage, SqlError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS scenario (
                id INTEGER PRIMARY KEY,
                family INTEGER NOT NULL,
                parent INTEGER,
                generation INTEGER NOT NULL,
                world TEXT NOT NULL,
                score REAL NOT NULL
            )",
            &[],
        )?;
        Ok(SqliteStorage { conn })
    }
}

/// Default is required for Specs resources. Default SqliteStorage just runs open_in_memory.
impl Default for SqliteStorage {
    fn default() -> Self {
        SqliteStorage::open_in_memory().unwrap()
    }
}

impl Storage for SqliteStorage {
    fn add_root_scenario(&mut self, world: World, score: f64) -> Result<Scenario, Box<Error>> {
        let txn = self.conn.transaction()?;
        let inserted = txn.execute(
            "INSERT INTO scenario (family, parent, generation, world, score)
                VALUES (?1, ?2, ?3, ?4, ?5)",
            &[&-1i64, &None::<i64>, &0i64, &world, &score],
        )?;
        if inserted != 1 {
            return Err(format!("Expected to insert 1 row but had {} row changes", inserted).into());
        }
        let id = txn.last_insert_rowid();
        let updated = txn.execute("UPDATE scenario SET family = ?1 WHERE id = ?1", &[&id])?;
        if updated != 1 {
            return Err(format!("Expected to update 1 row but had {} row changes", updated).into());
        }
        txn.commit()?;
        Ok(Scenario {
            id,
            family: id,
            parent: None,
            generation: 0,
            world,
            score,
        })
    }

    fn add_child_scenario(
        &mut self,
        world: World,
        score: f64,
        parent: &Scenario,
    ) -> Result<Scenario, Box<Error>> {
        let generation = parent.generation + 1;
        let inserted = self.conn.execute(
            "INSERT INTO scenario (family, parent, generation, world, score)
                VALUES (?1, ?2, ?3, ?4, ?5)",
            &[&parent.family, &Some(parent.id), &generation, &world, &score],
        )?;
        if inserted != 1 {
            return Err(format!("Expected to insert 1 row but had {} row changes", inserted).into());
        }
        let id = self.conn.last_insert_rowid();
        Ok(Scenario {
            id,
            family: parent.family,
            parent: Some(parent.id),
            generation,
            world,
            score,
        })
    }

    fn num_scenarios(&mut self) -> Result<i64, Box<Error>> {
        self.conn
            .query_row("SELECT COUNT(*) FROM scenario", &[], |row| row.get(0))
            .map_err(|err| err.into())
    }

    fn get_nth_scenario_by_score(&mut self, index: i64) -> Result<Option<Scenario>, Box<Error>> {
        let query_result = self.conn
            .query_row(
                "SELECT id, family, parent, generation, world, score
                    FROM scenario
                    ORDER BY score DESC,
                             id ASC
                    LIMIT 1
                    OFFSET ?",
                &[&index],
                |row| Scenario {
                    id: row.get(0),
                    family: row.get(1),
                    parent: row.get(2),
                    generation: row.get(3),
                    world: row.get(4),
                    score: row.get(5),
                },
            );
        match query_result {
            Ok(scenario) => Ok(Some(scenario)),
            Err(SqlError::QueryReturnedNoRows) => Ok(None),
            Err(any_other_error) => Err(any_other_error.into()),
        }
    }
}

impl ToSql for World {
    fn to_sql(&self) -> Result<ToSqlOutput, SqlError> {
        match serde_json::to_string(self) {
            Ok(s) => Ok(ToSqlOutput::Owned(SqlValue::Text(s))),
            Err(err) => Err(SqlError::ToSqlConversionFailure(err.into())),
        }
    }
}

impl FromSql for World {
    fn column_result(value: SqlValueRef) -> Result<Self, FromSqlError> {
        let serialized = match value {
            SqlValueRef::Text(serialized) => serialized,
            _ => return Err(FromSqlError::InvalidType),
        };
        serde_json::from_str(serialized).map_err(|err| FromSqlError::Other(err.into()))
    }
}

#[cfg(test)]
mod tests {
    use xsecurelock_saver::engine::components::physics::Vector;

    use super::*;
    use model::Planet;

    #[test]
    fn test_open_in_memory() {
        SqliteStorage::open_in_memory().unwrap();
    }

    #[test]
    fn test_add_root() {
        let mut storage = SqliteStorage::open_in_memory().unwrap();
        let world = World {
            planets: vec![Planet {
                    position: Vector::new(0., 0.),
                    velocity: Vector::new(0., 0.),
                    mass: 1.,
            }],
        };
        let scenario = storage.add_root_scenario(world.clone(), 54.).unwrap();
        assert_eq!(scenario.id, scenario.family);
        assert_eq!(scenario.parent, None);
        assert_eq!(scenario.generation, 0);
        assert_eq!(scenario.world, world);
        assert_eq!(scenario.score, 54.);

        let values: (i64, i64, Option<i64>, i64, World, f64) = storage
            .conn
            .query_row(
                "SELECT id, family, parent, generation, world, score
                    FROM scenario
                    WHERE id = ?1",
                &[&scenario.id],
                |row| (row.get(0), row.get(1), row.get(2), row.get(3), row.get(4), row.get(5)),
            ).unwrap();
        assert_eq!(values, (scenario.id, scenario.id, None, 0, world, 54.));
    }

    #[test]
    fn test_add_child() {
        let mut storage = SqliteStorage::open_in_memory().unwrap();
        let parent = Scenario {
            id: 34,
            family: 87,
            parent: Some(60),
            generation: 10,
            world: World { planets: vec![] },
            score: 3609.,
        };
        let world = World {
            planets: vec![Planet {
                position: Vector::new(0., 0.),
                velocity: Vector::new(0., 0.),
                mass: 1.,
            }],
        };
        let scenario = storage.add_child_scenario(world.clone(), 987., &parent).unwrap();
        assert_eq!(scenario.family, parent.family);
        assert_eq!(scenario.parent, Some(parent.id));
        assert_eq!(scenario.generation, parent.generation + 1);
        assert_eq!(scenario.world, world);
        assert_eq!(scenario.score, 987.);

        let values: (i64, i64, Option<i64>, i64, World, f64) = storage.conn.query_row(
            "SELECT id, family, parent, generation, world, score
                FROM scenario
                WHERE id = ?1",
            &[&scenario.id],
            |row| (row.get(0), row.get(1), row.get(2), row.get(3), row.get(4), row.get(5)))
            .unwrap();
        assert_eq!(
            values,
            (scenario.id, parent.family, Some(parent.id), parent.generation + 1, world, 987.),
        );

    }

    #[test]
    fn test_num_scenarios_empty() {
        let mut storage = SqliteStorage::open_in_memory().unwrap();
        assert_eq!(storage.num_scenarios().unwrap(), 0);
    }

    #[test]
    fn test_num_scenarios() {
        let mut storage = SqliteStorage::open_in_memory().unwrap();
        let world1 = World {
            planets: vec![Planet {
                position: Vector::new(0., 0.),
                velocity: Vector::new(0., 0.),
                mass: 1.,
            }],
        };
        let world2 = World { planets: vec![] };
        let world3 = World {
            planets: vec![Planet {
                position: Vector::new(80., 0.),
                velocity: Vector::new(25., 30.),
                mass: 15.,
            }],
        };

        {
            let mut add_row = storage
                .conn
                .prepare(
                    "INSERT INTO scenario (family, parent, generation, world, score)
                        VALUES (?1, ?2, ?3, ?4, ?5)",
                ).unwrap();
            add_row.execute(&[&36i64, &Some(54i64), &10i64, &world1, &90f64]).unwrap();
            add_row.execute(&[&580i64, &Some(908i64), &5i64, &world2, &763f64]).unwrap();
            add_row.execute(&[&170i64, &None::<i64>, &32i64, &world3, &66f64]).unwrap();
            add_row.execute(&[&80i64, &Some(6i64), &15i64, &world2, &90f64]).unwrap();
        }

        assert_eq!(storage.num_scenarios().unwrap(), 4);
    }

    #[test]
    fn test_get_nth_scenario_by_score() {
        let mut storage = SqliteStorage::open_in_memory().unwrap();
        let world1 = World {
            planets: vec![Planet {
                position: Vector::new(0., 0.),
                velocity: Vector::new(0., 0.),
                mass: 1.,
            }],
        };
        let world2 = World { planets: vec![] };
        let world3 = World {
            planets: vec![Planet {
                position: Vector::new(80., 0.),
                velocity: Vector::new(25., 30.),
                mass: 15.,
            }],
        };

        {
            let mut add_row = storage
                .conn
                .prepare(
                    "INSERT INTO scenario (family, parent, generation, world, score)
                        VALUES (?1, ?2, ?3, ?4, ?5)",
                ).unwrap();
            add_row.execute(&[&36i64, &Some(54i64), &10i64, &world1, &90f64]).unwrap();
            add_row.execute(&[&580i64, &Some(908i64), &5i64, &world2, &763f64]).unwrap();
            add_row.execute(&[&170i64, &None::<i64>, &32i64, &world3, &66f64]).unwrap();
            add_row.execute(&[&80i64, &Some(6i64), &15i64, &world2, &90f64]).unwrap();
        }

        let scenario = storage.get_nth_scenario_by_score(0).unwrap().unwrap();
        assert_eq!(scenario.family, 580);
        assert_eq!(scenario.parent, Some(908));
        assert_eq!(scenario.generation, 5);
        assert_eq!(scenario.world, world2);
        assert_eq!(scenario.score, 763.);

        let scenario = storage.get_nth_scenario_by_score(1).unwrap().unwrap();
        assert_eq!(scenario.family, 36);
        assert_eq!(scenario.parent, Some(54));
        assert_eq!(scenario.generation, 10);
        assert_eq!(scenario.world, world1);
        assert_eq!(scenario.score, 90.);

        let scenario = storage.get_nth_scenario_by_score(2).unwrap().unwrap();
        assert_eq!(scenario.family, 80);
        assert_eq!(scenario.parent, Some(6));
        assert_eq!(scenario.generation, 15);
        assert_eq!(scenario.world, world2);
        assert_eq!(scenario.score, 90.);

        let scenario = storage.get_nth_scenario_by_score(3).unwrap().unwrap();
        assert_eq!(scenario.family, 170);
        assert_eq!(scenario.parent, None);
        assert_eq!(scenario.generation, 32);
        assert_eq!(scenario.world, world3);
        assert_eq!(scenario.score, 66.);

        assert!(storage.get_nth_scenario_by_score(4).unwrap().is_none());
    }
}
