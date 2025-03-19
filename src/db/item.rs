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
    pub closing_code: Option<u8>,
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
            closing_code: None,
        }
    }

    pub fn with_target_time(
        content: String,
        action: String,
        category: String,
        target_time: Option<i64>,
    ) -> Self {
        let mut item = Self::new(action, category, content);
        item.target_time = target_time;
        item
    }

    // For backfills
    pub fn with_create_time(
        content: String,
        action: String,
        category: String,
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
        assert!(item.closing_code.is_none());
    }

    #[test]
    fn test_with_target_time() {
        let target_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            + 3600; // One hour in the future

        let item = Item::with_target_time(
            "content".to_string(),
            "action".to_string(),
            "category".to_string(),
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
            "content".to_string(),
            "action".to_string(),
            "category".to_string(),
            create_time,
        );

        assert_eq!(item.create_time, create_time);
    }
}
