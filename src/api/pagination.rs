use serde::{Deserialize, Serialize};

/// Pagination query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    /// Page number (1-based)
    pub page: Option<u32>,
    /// Items per page
    pub per_page: Option<u32>,
    /// Sort field
    pub sort_by: Option<String>,
    /// Sort order: asc or desc
    pub sort_order: Option<String>,
    /// Continuation token for cursor-based pagination
    pub cursor: Option<String>,
}

impl PaginationParams {
    /// Returns the page number, defaulting to 1.
    pub fn page_number(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    /// Returns items per page, clamped between 1 and max.
    pub fn items_per_page(&self, max: u32) -> u32 {
        self.per_page.unwrap_or(25).clamp(1, max)
    }

    /// Returns the offset for SQL-style pagination.
    pub fn offset(&self, max_per_page: u32) -> u32 {
        let page = self.page_number();
        let per_page = self.items_per_page(max_per_page);
        page.saturating_sub(1).saturating_mul(per_page)
    }

    /// Returns true if sort order is descending.
    pub fn is_descending(&self) -> bool {
        self.sort_order
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case("desc"))
            .unwrap_or(false)
    }
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            per_page: Some(25),
            sort_by: None,
            sort_order: None,
            cursor: None,
        }
    }
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub pagination: PaginationInfo,
}

/// Pagination metadata in responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub per_page: u32,
    pub total_items: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_previous: bool,
    pub next_cursor: Option<String>,
}

impl PaginationInfo {
    /// Create pagination info from counts.
    pub fn new(page: u32, per_page: u32, total_items: u64) -> Self {
        let total_pages = if per_page == 0 {
            0
        } else {
            // Use u64 arithmetic to avoid truncation, then clamp to u32
            let pages = total_items.saturating_add(per_page as u64 - 1) / per_page as u64;
            pages.min(u32::MAX as u64) as u32
        };
        Self {
            page,
            per_page,
            total_items,
            total_pages,
            has_next: page < total_pages,
            has_previous: page > 1,
            next_cursor: None,
        }
    }
}

/// Apply pagination to a vector of items.
pub fn paginate<T: Clone + Serialize>(items: &[T], params: &PaginationParams) -> PaginatedResponse<T> {
    let max_per_page = 100;
    let per_page = params.items_per_page(max_per_page) as usize;
    let offset = params.offset(max_per_page) as usize;
    let total_items = items.len() as u64;

    let page_items: Vec<T> = items.iter().skip(offset).take(per_page).cloned().collect();

    PaginatedResponse {
        items: page_items,
        pagination: PaginationInfo::new(
            params.page_number(),
            per_page as u32,
            total_items,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_params_defaults() {
        let params = PaginationParams::default();
        assert_eq!(params.page_number(), 1);
        assert_eq!(params.items_per_page(100), 25);
        assert_eq!(params.offset(100), 0);
        assert!(!params.is_descending());
    }

    #[test]
    fn test_pagination_params_clamp() {
        let params = PaginationParams {
            page: Some(3),
            per_page: Some(500),
            sort_by: None,
            sort_order: None,
            cursor: None,
        };
        assert_eq!(params.items_per_page(100), 100);
        assert_eq!(params.offset(100), 200);
    }

    #[test]
    fn test_pagination_info() {
        let info = PaginationInfo::new(2, 10, 35);
        assert_eq!(info.total_pages, 4);
        assert!(info.has_next);
        assert!(info.has_previous);
    }

    #[test]
    fn test_paginate() {
        let items: Vec<i32> = (1..=50).collect();
        let params = PaginationParams {
            page: Some(2),
            per_page: Some(10),
            sort_by: None,
            sort_order: None,
            cursor: None,
        };
        let result = paginate(&items, &params);
        assert_eq!(result.items.len(), 10);
        assert_eq!(result.items[0], 11);
        assert_eq!(result.pagination.total_items, 50);
        assert_eq!(result.pagination.total_pages, 5);
    }
}
