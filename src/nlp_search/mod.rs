// Natural Language Search - Parse natural language queries for VM filtering
//
// Supports queries like:
// - "vms using more than 80% cpu"
// - "stopped vms created last week"
// - "ubuntu vms in production namespace"

use std::sync::LazyLock;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub filters: Vec<Filter>,
    pub sort_by: Option<SortField>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Filter {
    Status(StatusFilter),
    CpuUsage(ComparisonFilter),
    MemoryUsage(ComparisonFilter),
    OsType(String),
    Namespace(String),
    NodeName(String),
    Tag(String),
    Name(String),
}

#[derive(Debug, Clone)]
pub enum StatusFilter { Running, Stopped, Failed, Any }

#[derive(Debug, Clone)]
pub struct ComparisonFilter {
    pub operator: Operator,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub enum Operator { GreaterThan, LessThan, Equal, GreaterOrEqual, LessOrEqual }

#[derive(Debug, Clone)]
pub enum SortField { Name, Cpu, Memory, Age, Status }

impl SearchQuery {
    pub fn parse(query: &str) -> Self {
        let q = query.to_lowercase();
        let mut filters = Vec::new();

        // Parse status
        if q.contains("running") { filters.push(Filter::Status(StatusFilter::Running)); }
        else if q.contains("stopped") || q.contains("shut") { filters.push(Filter::Status(StatusFilter::Stopped)); }
        else if q.contains("failed") || q.contains("error") { filters.push(Filter::Status(StatusFilter::Failed)); }

        // Parse CPU usage
        if let Some(cpu) = parse_resource_threshold(&q, "cpu") {
            filters.push(Filter::CpuUsage(cpu));
        }

        // Parse memory usage
        if let Some(mem) = parse_resource_threshold(&q, "memory") {
            filters.push(Filter::MemoryUsage(mem));
        }

        // Parse OS type
        for os in &["ubuntu", "centos", "fedora", "rhel", "debian", "windows", "alpine"] {
            if q.contains(os) { filters.push(Filter::OsType(os.to_string())); break; }
        }

        // Parse namespace
        if let Some(ns) = parse_namespace(&q) {
            filters.push(Filter::Namespace(ns));
        }

        // Parse node
        if let Some(node) = parse_node(&q) {
            filters.push(Filter::NodeName(node));
        }

        // Parse sort
        let sort_by = if q.contains("sort by cpu") || q.contains("by cpu") { Some(SortField::Cpu) }
        else if q.contains("sort by memory") || q.contains("by memory") { Some(SortField::Memory) }
        else if q.contains("sort by name") || q.contains("by name") { Some(SortField::Name) }
        else if q.contains("sort by age") || q.contains("by age") { Some(SortField::Age) }
        else { None };

        // Parse limit
        static LIMIT_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?:top|first|limit)\s+(\d+)").unwrap());
        let limit = LIMIT_RE.captures(&q)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse().ok());

        SearchQuery { filters, sort_by, limit }
    }

    pub fn is_empty(&self) -> bool {
        self.filters.is_empty() && self.sort_by.is_none() && self.limit.is_none()
    }

    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        for filter in &self.filters {
            match filter {
                Filter::Status(s) => parts.push(format!("status={:?}", s)),
                Filter::CpuUsage(c) => parts.push(format!("cpu {:?} {:.0}%", c.operator, c.value)),
                Filter::MemoryUsage(c) => parts.push(format!("memory {:?} {:.0}%", c.operator, c.value)),
                Filter::OsType(os) => parts.push(format!("os={}", os)),
                Filter::Namespace(ns) => parts.push(format!("namespace={}", ns)),
                Filter::NodeName(n) => parts.push(format!("node={}", n)),
                Filter::Tag(t) => parts.push(format!("tag={}", t)),
                Filter::Name(n) => parts.push(format!("name~{}", n)),
            }
        }
        if let Some(ref s) = self.sort_by { parts.push(format!("sort={:?}", s)); }
        if let Some(l) = self.limit { parts.push(format!("limit={}", l)); }
        parts.join(", ")
    }
}

fn parse_resource_threshold(query: &str, resource: &str) -> Option<ComparisonFilter> {
    use std::collections::HashMap;
    use std::sync::Mutex;

    // Cache compiled regexes per resource name
    type PatternCache = HashMap<String, Vec<(Regex, Operator)>>;
    static CACHE: LazyLock<Mutex<PatternCache>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    let patterns = {
        let mut cache = CACHE.lock().ok()?;
        cache.entry(resource.to_string()).or_insert_with(|| {
            vec![
                (Regex::new(&format!(r"{}\s*(?:usage\s*)?(?:>|more than|above|over)\s*(\d+)", resource)).unwrap(), Operator::GreaterThan),
                (Regex::new(&format!(r"{}\s*(?:usage\s*)?(?:<|less than|below|under)\s*(\d+)", resource)).unwrap(), Operator::LessThan),
                (Regex::new(&format!(r"(?:>|more than|above|over)\s*(\d+)%?\s*{}", resource)).unwrap(), Operator::GreaterThan),
                (Regex::new(&format!(r"(?:<|less than|below|under)\s*(\d+)%?\s*{}", resource)).unwrap(), Operator::LessThan),
            ]
        }).clone()
    };

    for (re, op) in &patterns {
        if let Some(caps) = re.captures(query) {
            if let Some(val) = caps.get(1).and_then(|m| m.as_str().parse::<f64>().ok()) {
                return Some(ComparisonFilter { operator: op.clone(), value: val });
            }
        }
    }
    None
}

fn parse_namespace(query: &str) -> Option<String> {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:in|namespace|ns)\s+(\S+)").unwrap());
    RE.captures(query)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

fn parse_node(query: &str) -> Option<String> {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:on|node)\s+([\w\-\.]+)").unwrap());
    RE.captures(query)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

impl std::fmt::Display for StatusFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::Running => write!(f, "Running"), Self::Stopped => write!(f, "Stopped"),
            Self::Failed => write!(f, "Failed"), Self::Any => write!(f, "Any") }
    }
}

impl std::fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::GreaterThan => write!(f, ">"), Self::LessThan => write!(f, "<"),
            Self::Equal => write!(f, "="), Self::GreaterOrEqual => write!(f, ">="), Self::LessOrEqual => write!(f, "<=") }
    }
}
