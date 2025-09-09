use crate::models::spring::{Value, DependencyCategories};

pub enum Lists {
    Values(Vec<Value>),
    Categories(Vec<DependencyCategories>),
}
