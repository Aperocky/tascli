use std::time::{
    SystemTime,
    UNIX_EPOCH,
};

use rusqlite::Row;

#[derive(Debug, Clone)]
pub struct Item {
    pub id: Option<i64>,
    pub action: String,
    pub category: String,
    pub content: String,
    pub create_time: i64,
    pub target_time: Option<i64>,
    pub modify_time: Option<i64>,
    pub closing_code: u8,
}

impl Item {
    pub fn new(action: String, category: String, content: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Self {
            id: None,
            action,
            category,
            content,
            create_time: now,
            target_time: None,
            modify_time: None,
            closing_code: 0,
        }
    }

    pub fn with_target_time(
        action: String,
        category: String,
        content: String,
        target_time: Option<i64>,
    ) -> Self {
        let mut item = Self::new(action, category, content);
        item.target_time = target_time;
        item
    }

    // For backfills
    pub fn with_create_time(
        action: String,
        category: String,
        content: String,
        create_time: i64,
    ) -> Self {
        let mut item = Self::new(action, category, content);
        item.create_time = create_time;
        item
    }

    pub fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            action: row.get("action")?,
            category: row.get("category")?,
            content: row.get("content")?,
            create_time: row.get("create_time")?,
            target_time: row.get("target_time")?,
            modify_time: row.get("modify_time")?,
            closing_code: row.get("closing_code")?,
        })
    }
}

// Query Struct for querying items from db
pub struct ItemQuery<'a> {
    pub action: Option<&'a str>,
    pub category: Option<&'a str>,
    pub create_time_min: Option<i64>,
    pub create_time_max: Option<i64>,
    pub target_time_min: Option<i64>,
    pub target_time_max: Option<i64>,
    pub closing_code: Option<u8>,
    pub limit: Option<usize>,
    pub offset_id: Option<i64>,
}

impl<'a> ItemQuery<'a> {
    pub fn new() -> Self {
        ItemQuery {
            action: None,
            category: None,
            create_time_min: None,
            create_time_max: None,
            target_time_min: None,
            target_time_max: None,
            closing_code: None,
            limit: None,
            offset_id: None,
        }
    }

    pub fn with_action(mut self, action: &'a str) -> Self {
        self.action = Some(action);
        self
    }

    pub fn with_category(mut self, category: &'a str) -> Self {
        self.category = Some(category);
        self
    }

    pub fn with_create_time_range(mut self, min: Option<i64>, max: Option<i64>) -> Self {
        self.create_time_min = min;
        self.create_time_max = max;
        self
    }

    pub fn with_target_time_range(mut self, min: Option<i64>, max: Option<i64>) -> Self {
        self.target_time_min = min;
        self.target_time_max = max;
        self
    }

    pub fn with_create_time_min(mut self, create_time_min: i64) -> Self {
        self.create_time_min = Some(create_time_min);
        self
    }

    pub fn with_create_time_max(mut self, create_time_max: i64) -> Self {
        self.create_time_max = Some(create_time_max);
        self
    }

    pub fn with_target_time_min(mut self, target_time_min: i64) -> Self {
        self.target_time_min = Some(target_time_min);
        self
    }

    pub fn with_target_time_max(mut self, target_time_max: i64) -> Self {
        self.target_time_max = Some(target_time_max);
        self
    }

    pub fn with_closing_code(mut self, closing_code: u8) -> Self {
        self.closing_code = Some(closing_code);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset_id(mut self, offset_id: i64) -> Self {
        self.offset_id = Some(offset_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_item() {
        let item = Item::new(
            "action".to_string(),
            "category".to_string(),
            "content".to_string(),
        );

        assert_eq!(item.action, "action");
        assert_eq!(item.category, "category");
        assert_eq!(item.content, "content");
        assert!(item.id.is_none());
        assert!(item.target_time.is_none());
        assert!(item.modify_time.is_none());
        assert_eq!(item.closing_code, 0);
    }

    #[test]
    fn test_with_target_time() {
        let target_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            + 3600; // One hour in the future

        let item = Item::with_target_time(
            "action".to_string(),
            "category".to_string(),
            "content".to_string(),
            Some(target_time),
        );

        assert_eq!(item.action, "action");
        assert_eq!(item.category, "category");
        assert_eq!(item.content, "content");
        assert_eq!(item.target_time, Some(target_time));
    }

    #[test]
    fn test_with_create_time() {
        let create_time = 1700000000;

        let item = Item::with_create_time(
            "action".to_string(),
            "category".to_string(),
            "content".to_string(),
            create_time,
        );

        assert_eq!(item.create_time, create_time);
    }

    #[test]
    fn test_item_query_builder() {
        // Test default values from new()
        let query = ItemQuery::new();
        assert_eq!(query.action, None);
        assert_eq!(query.category, None);
        assert_eq!(query.create_time_min, None);
        assert_eq!(query.create_time_max, None);
        assert_eq!(query.target_time_min, None);
        assert_eq!(query.target_time_max, None);
        assert_eq!(query.closing_code, None);
        assert_eq!(query.limit, None);
        assert_eq!(query.offset_id, None);

        let query = ItemQuery::new().with_action("task");
        assert_eq!(query.action, Some("task"));

        let query = ItemQuery::new().with_create_time_range(Some(1000), Some(2000));
        assert_eq!(query.create_time_min, Some(1000));
        assert_eq!(query.create_time_max, Some(2000));

        let query = ItemQuery::new().with_target_time_range(Some(3000), Some(4000));
        assert_eq!(query.target_time_min, Some(3000));
        assert_eq!(query.target_time_max, Some(4000));

        let query = ItemQuery::new().with_closing_code(1);
        assert_eq!(query.closing_code, Some(1));

        let query = ItemQuery::new().with_limit(100);
        assert_eq!(query.limit, Some(100));

        // Test chaining
        let query = ItemQuery::new()
            .with_action("record")
            .with_category("feeding")
            .with_create_time_min(40000)
            .with_limit(100);

        assert_eq!(query.action, Some("record"));
        assert_eq!(query.category, Some("feeding"));
        assert_eq!(query.create_time_min, Some(40000));
        assert_eq!(query.create_time_max, None);
        assert_eq!(query.target_time_min, None);
        assert_eq!(query.target_time_max, None);
        assert_eq!(query.closing_code, None);
        assert_eq!(query.limit, Some(100));
        assert_eq!(query.offset_id, None);
    }
}
