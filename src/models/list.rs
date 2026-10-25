use crate::models::spring::{DependencyCategories, Value};

pub enum Lists {
    Values(Vec<Value>),
    Categories(Vec<DependencyCategories>),
}
