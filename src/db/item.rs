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
    #[allow(dead_code)]
    pub modify_time: Option<i64>,
    pub status: u8,
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
            status: 0,
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
            status: row.get("status")?,
        })
    }
}

// Query Struct for querying items from db
#[derive(Debug)]
pub struct ItemQuery<'a> {
    pub action: Option<&'a str>,
    pub category: Option<&'a str>,
    pub content_like: Option<&'a str>,
    pub create_time_min: Option<i64>,
    pub create_time_max: Option<i64>,
    pub target_time_min: Option<i64>,
    pub target_time_max: Option<i64>,
    pub statuses: Option<Vec<u8>>,
    pub limit: Option<usize>,
    pub offset: Offset,
    pub order_by: Option<&'a str>,
}

#[derive(Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Offset {
    None,
    Id(i64),
    CreateTime(i64),
    TargetTime(i64),
}

#[allow(dead_code)]
impl<'a> ItemQuery<'a> {
    pub fn new() -> Self {
        ItemQuery {
            action: None,
            category: None,
            content_like: None,
            create_time_min: None,
            create_time_max: None,
            target_time_min: None,
            target_time_max: None,
            statuses: None,
            limit: None,
            offset: Offset::None,
            order_by: None,
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

    pub fn with_content_like(mut self, content: &'a str) -> Self {
        self.content_like = Some(content);
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

    pub fn with_statuses(mut self, statuses: Vec<u8>) -> Self {
        self.statuses = Some(statuses);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: Offset) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_order_by(mut self, order_by: &'a str) -> Self {
        self.order_by = Some(order_by);
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
        assert_eq!(item.status, 0);
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
        assert_eq!(query.statuses, None);
        assert_eq!(query.limit, None);
        assert_eq!(query.offset, Offset::None);
        assert_eq!(query.order_by, None);

        let query = ItemQuery::new().with_action("task");
        assert_eq!(query.action, Some("task"));

        let query = ItemQuery::new().with_create_time_range(Some(1000), Some(2000));
        assert_eq!(query.create_time_min, Some(1000));
        assert_eq!(query.create_time_max, Some(2000));

        let query = ItemQuery::new().with_target_time_range(Some(3000), Some(4000));
        assert_eq!(query.target_time_min, Some(3000));
        assert_eq!(query.target_time_max, Some(4000));

        let query = ItemQuery::new().with_statuses(vec![0]);
        assert_eq!(query.statuses, Some(vec![0]));

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
        assert_eq!(query.statuses, None);
        assert_eq!(query.limit, Some(100));
        assert_eq!(query.offset, Offset::None);
        assert_eq!(query.order_by, None);
    }
}
