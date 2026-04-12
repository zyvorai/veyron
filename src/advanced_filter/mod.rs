// Advanced Filtering - Multi-criteria filtering with boolean logic

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedFilter {
    pub conditions: Vec<FilterCondition>,
    pub logic: FilterLogic,
    pub sort: Option<SortConfig>,
    pub pagination: Option<PaginationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCondition {
    pub field: FilterField,
    pub operator: FilterOperator,
    pub value: String,
    pub negate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FilterField {
    Name, Status, Namespace, Node, CpuCores, Memory, Age, Labels, Annotations, Ready,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    Equals, NotEquals, Contains, StartsWith, EndsWith,
    GreaterThan, LessThan, GreaterOrEqual, LessOrEqual,
    In, NotIn, Exists, Regex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterLogic { And, Or }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortConfig {
    pub field: FilterField,
    pub ascending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationConfig {
    pub page: usize,
    pub page_size: usize,
}

impl AdvancedFilter {
    pub fn new() -> Self {
        Self { conditions: Vec::new(), logic: FilterLogic::And, sort: None, pagination: None }
    }

    pub fn add_condition(&mut self, condition: FilterCondition) {
        self.conditions.push(condition);
    }

    pub fn with_sort(mut self, field: FilterField, ascending: bool) -> Self {
        self.sort = Some(SortConfig { field, ascending });
        self
    }

    pub fn with_pagination(mut self, page: usize, page_size: usize) -> Self {
        self.pagination = Some(PaginationConfig { page, page_size });
        self
    }

    pub fn matches(&self, values: &std::collections::HashMap<String, String>) -> bool {
        let results: Vec<bool> = self.conditions.iter().map(|c| {
            let field_name = format!("{:?}", c.field).to_lowercase();
            let val = values.get(&field_name).map(|s| s.as_str()).unwrap_or("");
            let matched = match c.operator {
                FilterOperator::Equals => val == c.value,
                FilterOperator::NotEquals => val != c.value,
                FilterOperator::Contains => val.contains(&c.value),
                FilterOperator::StartsWith => val.starts_with(&c.value),
                FilterOperator::EndsWith => val.ends_with(&c.value),
                FilterOperator::GreaterThan => val.parse::<f64>().unwrap_or(0.0) > c.value.parse::<f64>().unwrap_or(0.0),
                FilterOperator::LessThan => val.parse::<f64>().unwrap_or(0.0) < c.value.parse::<f64>().unwrap_or(0.0),
                FilterOperator::GreaterOrEqual => val.parse::<f64>().unwrap_or(0.0) >= c.value.parse::<f64>().unwrap_or(0.0),
                FilterOperator::LessOrEqual => val.parse::<f64>().unwrap_or(0.0) <= c.value.parse::<f64>().unwrap_or(0.0),
                FilterOperator::In => c.value.split(',').any(|v| v.trim() == val),
                FilterOperator::NotIn => !c.value.split(',').any(|v| v.trim() == val),
                FilterOperator::Exists => !val.is_empty(),
                FilterOperator::Regex => regex::Regex::new(&c.value).map(|re| re.is_match(val)).unwrap_or(false),
            };
            if c.negate { !matched } else { matched }
        }).collect();

        match self.logic {
            FilterLogic::And => results.iter().all(|&r| r),
            FilterLogic::Or => results.iter().any(|&r| r),
        }
    }

    pub fn is_empty(&self) -> bool { self.conditions.is_empty() }
}

impl Default for AdvancedFilter {
    fn default() -> Self { Self::new() }
}
