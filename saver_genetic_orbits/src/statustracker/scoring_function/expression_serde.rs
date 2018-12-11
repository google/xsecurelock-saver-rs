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

mod ser {
    use serde::ser::{Serialize, Serializer};

    use crate::statustracker::scoring_function::Expression;

    impl Serialize for Expression {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: Serializer,
        {
            serializer.serialize_str(&format!("{}", self))
        }
    }
}

mod de {
    use serde::de::{Deserialize, Deserializer, Visitor, Error};
    use std::fmt;
    
    use crate::statustracker::scoring_function::Expression;
    
    impl<'de> Deserialize<'de> for Expression {
        fn deserialize<D>(deserializer: D) -> Result<Expression, D::Error>
            where D: Deserializer<'de>,
        {
            deserializer.deserialize_str(ExpressionVisitor)
        }
    }

    struct ExpressionVisitor;

    impl<'de> Visitor<'de> for ExpressionVisitor {
        type Value = Expression;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a math expression")
        }

        fn visit_i8<E: Error>(self, v: i8) -> Result<Self::Value, E> {
            Ok(Expression::Constant(v as f64))
        }
        fn visit_i16<E: Error>(self, v: i16) -> Result<Self::Value, E> {
            Ok(Expression::Constant(v as f64))
        }
        fn visit_i32<E: Error>(self, v: i32) -> Result<Self::Value, E> {
            Ok(Expression::Constant(v as f64))
        }
        fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Expression::Constant(v as f64))
        }
        fn visit_i128<E: Error>(self, v: i128) -> Result<Self::Value, E> {
            Ok(Expression::Constant(v as f64))
        }
        fn visit_u8<E: Error>(self, v: u8) -> Result<Self::Value, E> {
            Ok(Expression::Constant(v as f64))
        }
        fn visit_u16<E: Error>(self, v: u16) -> Result<Self::Value, E> {
            Ok(Expression::Constant(v as f64))
        }
        fn visit_u32<E: Error>(self, v: u32) -> Result<Self::Value, E> {
            Ok(Expression::Constant(v as f64))
        }
        fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Expression::Constant(v as f64))
        }
        fn visit_u128<E: Error>(self, v: u128) -> Result<Self::Value, E> {
            Ok(Expression::Constant(v as f64))
        }
        fn visit_f32<E: Error>(self, v: f32) -> Result<Self::Value, E> {
            Ok(Expression::Constant(v as f64))
        }
        fn visit_f64<E: Error>(self, v: f64) -> Result<Self::Value, E> {
            Ok(Expression::Constant(v))
        }
        
        fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
            v.parse().map_err(E::custom)
        }

        fn visit_borrowed_str<E: Error>(self, v: &'de str) -> Result<Self::Value, E> {
            self.visit_str(v)
        }

        fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E> {
            self.visit_str(&v)
        }
    }
}
