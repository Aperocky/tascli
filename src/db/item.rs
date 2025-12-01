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
    pub cron_schedule: Option<String>,
    pub human_schedule: Option<String>,
    pub recurring_task_id: Option<i64>,
    pub good_until: Option<i64>,
}

const RECURRING_TASK: &str = "recurring_task";
const RECURRING_TASK_RECORD: &str = "recurring_task_record";

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
            cron_schedule: None,
            human_schedule: None,
            recurring_task_id: None,
            good_until: None,
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

    pub fn create_recurring_task(
        category: String,
        content: String,
        cron_schedule: String,
        human_schedule: String,
    ) -> Self {
        let mut item = Self::new(RECURRING_TASK.to_string(), category, content);
        item.cron_schedule = Some(cron_schedule.to_string());
        item.human_schedule = Some(human_schedule.to_string());
        item
    }

    pub fn create_recurring_record(
        category: String,
        content: String,
        recurring_task_id: i64,
        good_until: i64,
    ) -> Self {
        let mut item = Self::new(RECURRING_TASK_RECORD.to_string(), category, content);
        item.recurring_task_id = Some(recurring_task_id);
        item.good_until = Some(good_until);
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
            cron_schedule: row.get("cron_schedule")?,
            human_schedule: row.get("human_schedule")?,
            recurring_task_id: row.get("recurring_task_id")?,
            good_until: row.get("good_until")?,
        })
    }
}

// Query Struct for querying items from db
#[derive(Debug)]
pub struct ItemQuery<'a> {
    pub actions: Option<Vec<&'a str>>,
    pub category: Option<&'a str>,
    pub content_like: Option<&'a str>,
    pub create_time_min: Option<i64>,
    pub create_time_max: Option<i64>,
    pub target_time_min: Option<i64>,
    pub target_time_max: Option<i64>,
    pub good_until_min: Option<i64>,
    pub good_until_max: Option<i64>,
    pub recurring_task_id: Option<i64>,
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
            actions: None,
            category: None,
            content_like: None,
            create_time_min: None,
            create_time_max: None,
            target_time_min: None,
            target_time_max: None,
            good_until_min: None,
            good_until_max: None,
            recurring_task_id: None,
            statuses: None,
            limit: None,
            offset: Offset::None,
            order_by: None,
        }
    }

    pub fn with_action(mut self, action: &'a str) -> Self {
        self.actions = Some(vec![action]);
        self
    }

    pub fn with_actions(mut self, actions: Vec<&'a str>) -> Self {
        self.actions = Some(actions);
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

    pub fn with_good_until_range(mut self, min: Option<i64>, max: Option<i64>) -> Self {
        self.good_until_min = min;
        self.good_until_max = max;
        self
    }

    pub fn with_good_until_min(mut self, good_until_min: i64) -> Self {
        self.good_until_min = Some(good_until_min);
        self
    }

    pub fn with_good_until_max(mut self, good_until_max: i64) -> Self {
        self.good_until_max = Some(good_until_max);
        self
    }

    pub fn with_recurring_task_id(mut self, recurring_task_id: i64) -> Self {
        self.recurring_task_id = Some(recurring_task_id);
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
        assert!(item.cron_schedule.is_none());
        assert!(item.human_schedule.is_none());
        assert!(item.recurring_task_id.is_none());
        assert!(item.good_until.is_none());
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
    fn test_create_recurring_task() {
        let item = Item::create_recurring_task(
            "work".to_string(),
            "Weekly standup".to_string(),
            "0 9 * * 1".to_string(),
            "Weekly Monday 9AM".to_string(),
        );

        assert_eq!(item.action, "recurring_task");
        assert_eq!(item.category, "work");
        assert_eq!(item.content, "Weekly standup");
        assert_eq!(item.cron_schedule, Some("0 9 * * 1".to_string()));
        assert_eq!(item.human_schedule, Some("Weekly Monday 9AM".to_string()));
        assert!(item.id.is_none());
        assert!(item.target_time.is_none());
        assert!(item.modify_time.is_none());
        assert_eq!(item.status, 0);
        assert!(item.recurring_task_id.is_none());
        assert!(item.good_until.is_none());
    }

    #[test]
    fn test_create_recurring_record() {
        let recurring_task_id = 42;
        let good_until = 1760000000; // Some future timestamp

        let item = Item::create_recurring_record(
            "work".to_string(),
            "Weekly standup completed".to_string(),
            recurring_task_id,
            good_until,
        );

        assert_eq!(item.action, "recurring_task_record");
        assert_eq!(item.category, "work");
        assert_eq!(item.content, "Weekly standup completed");
        assert_eq!(item.recurring_task_id, Some(recurring_task_id));
        assert_eq!(item.good_until, Some(good_until));
        assert!(item.id.is_none());
        assert!(item.target_time.is_none());
        assert!(item.modify_time.is_none());
        assert_eq!(item.status, 0);
        assert!(item.cron_schedule.is_none());
        assert!(item.human_schedule.is_none());
    }

    #[test]
    fn test_item_query_builder() {
        // Test default values from new()
        let query = ItemQuery::new();
        assert_eq!(query.actions, None);
        assert_eq!(query.category, None);
        assert_eq!(query.create_time_min, None);
        assert_eq!(query.create_time_max, None);
        assert_eq!(query.target_time_min, None);
        assert_eq!(query.target_time_max, None);
        assert_eq!(query.good_until_min, None);
        assert_eq!(query.good_until_max, None);
        assert_eq!(query.recurring_task_id, None);
        assert_eq!(query.statuses, None);
        assert_eq!(query.limit, None);
        assert_eq!(query.offset, Offset::None);
        assert_eq!(query.order_by, None);

        let query = ItemQuery::new().with_action("task");
        assert_eq!(query.actions, Some(vec!["task"]));

        let query = ItemQuery::new().with_actions(vec!["task", "record"]);
        assert_eq!(query.actions, Some(vec!["task", "record"]));

        let query = ItemQuery::new().with_create_time_range(Some(1000), Some(2000));
        assert_eq!(query.create_time_min, Some(1000));
        assert_eq!(query.create_time_max, Some(2000));

        let query = ItemQuery::new().with_target_time_range(Some(3000), Some(4000));
        assert_eq!(query.target_time_min, Some(3000));
        assert_eq!(query.target_time_max, Some(4000));

        let query = ItemQuery::new().with_good_until_range(Some(5000), Some(6000));
        assert_eq!(query.good_until_min, Some(5000));
        assert_eq!(query.good_until_max, Some(6000));

        let query = ItemQuery::new().with_good_until_min(7000);
        assert_eq!(query.good_until_min, Some(7000));
        assert_eq!(query.good_until_max, None);

        let query = ItemQuery::new().with_good_until_max(8000);
        assert_eq!(query.good_until_min, None);
        assert_eq!(query.good_until_max, Some(8000));

        let query = ItemQuery::new().with_recurring_task_id(42);
        assert_eq!(query.recurring_task_id, Some(42));

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

        assert_eq!(query.actions, Some(vec!["record"]));
        assert_eq!(query.category, Some("feeding"));
        assert_eq!(query.create_time_min, Some(40000));
        assert_eq!(query.create_time_max, None);
        assert_eq!(query.target_time_min, None);
        assert_eq!(query.target_time_max, None);
        assert_eq!(query.good_until_min, None);
        assert_eq!(query.good_until_max, None);
        assert_eq!(query.recurring_task_id, None);
        assert_eq!(query.statuses, None);
        assert_eq!(query.limit, Some(100));
        assert_eq!(query.offset, Offset::None);
        assert_eq!(query.order_by, None);

        // Test chaining with recurring task fields
        let query = ItemQuery::new()
            .with_action("recurring_task_record")
            .with_recurring_task_id(42)
            .with_good_until_min(10000)
            .with_good_until_max(20000);

        assert_eq!(query.actions, Some(vec!["recurring_task_record"]));
        assert_eq!(query.recurring_task_id, Some(42));
        assert_eq!(query.good_until_min, Some(10000));
        assert_eq!(query.good_until_max, Some(20000));
    }
}
