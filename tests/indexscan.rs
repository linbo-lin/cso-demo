use cso_demo::datum::Datum;
use cso_demo::expression::{And, ScalarExpression};
use cso_demo::expression::{ColumnVar, IsNull};
use cso_demo::metadata::CachedMdProvider;
use cso_demo::metadata::MdAccessor;
use cso_demo::metadata::{MdCache, Metadata};
use cso_demo::operator::logical_filter::LogicalFilter;
use cso_demo::operator::logical_index_scan::IndexDesc;
use cso_demo::operator::logical_project::LogicalProject;
use cso_demo::operator::logical_scan::{LogicalScan, TableDesc};
use cso_demo::operator::physical_filter::PhysicalFilter;
use cso_demo::operator::physical_index_scan::PhysicalIndexScan;
use cso_demo::operator::physical_project::PhysicalProject;
use cso_demo::operator::physical_scan::PhysicalScan;
use cso_demo::operator::physical_sort::{OrderSpec, Ordering, PhysicalSort};
use cso_demo::property::sort_property::SortProperty;
use cso_demo::property::PhysicalProperties;
use cso_demo::rule::create_rule_set;
use cso_demo::statistics::{
    Bucket, ColumnMetadata, ColumnStats, Histogram, IndexInfo, IndexMd, IndexType, RelationMetadata, RelationStats,
};
use cso_demo::{LogicalPlan, Optimizer, Options, PhysicalPlan};
use std::rc::Rc;

fn logical_scan() -> LogicalPlan {
    let mdid = 2;
    let table_desc = TableDesc::new(mdid);
    let output_columns = vec![ColumnVar::new(0), ColumnVar::new(1), ColumnVar::new(2)];

    let scan = LogicalScan::new(table_desc, output_columns);
    LogicalPlan::new(Rc::new(scan), vec![], vec![])
}

fn logical_filter(input: Vec<LogicalPlan>, id_1: u32, id_2: Option<u32>) -> LogicalPlan {
    let predicate = IsNull::new(Box::new(ColumnVar::new(id_1)));
    let predicate = match id_2 {
        Some(id) => {
            let predicate_2 = IsNull::new(Box::new(ColumnVar::new(id)));
            let predicate = And::new(vec![Rc::new(predicate), Rc::new(predicate_2)]);
            Rc::new(predicate) as Rc<dyn ScalarExpression>
        }
        None => Rc::new(predicate) as Rc<dyn ScalarExpression>,
    };

    let filter = LogicalFilter::new(predicate);
    LogicalPlan::new(Rc::new(filter), input, vec![])
}

fn logical_project(inputs: Vec<LogicalPlan>) -> LogicalPlan {
    let project = vec![
        Rc::new(ColumnVar::new(1)) as Rc<dyn ScalarExpression>,
        Rc::new(ColumnVar::new(2)) as Rc<dyn ScalarExpression>,
    ];
    let project = LogicalProject::new(project);
    LogicalPlan::new(Rc::new(project), inputs, vec![])
}

fn required_properties(id: u32) -> Rc<PhysicalProperties> {
    let order = OrderSpec {
        order_desc: vec![Ordering {
            key: ColumnVar::new(id),
            ascending: true,
            nulls_first: true,
        }],
    };
    let sort_property = SortProperty::with_order(order);
    PhysicalProperties::with_property(Box::new(sort_property))
}

fn md_cache() -> MdCache {
    // mdids
    let relation_stats_id = 1;
    let relation_md_id = 2;
    let column_stats_id = 3;
    let index_md_id = 4;

    // relation stats
    let relation_stats = RelationStats::new("t1".to_string(), 9011, false, vec![column_stats_id]);
    let boxed_relation_stats = Box::new(relation_stats.clone()) as Box<dyn Metadata>;

    // index metadata
    let index_md = IndexMd::new(
        4,
        "IDX_1".to_string(),
        vec![ColumnVar::new(0)],
        vec![ColumnVar::new(0), ColumnVar::new(1), ColumnVar::new(2)],
    );
    let boxed_index_md = Box::new(index_md) as Box<dyn Metadata>;

    // relation metadata
    let column_md = vec![
        ColumnMetadata::new("c1".to_string(), 1, true, 4, Datum::I32(0)),
        ColumnMetadata::new("c2".to_string(), 2, true, 4, Datum::I32(0)),
        ColumnMetadata::new("c3".to_string(), 3, false, 4, Datum::I32(0)),
        ColumnMetadata::new("c4".to_string(), 4, false, 4, Datum::I32(0)),
        ColumnMetadata::new("c5".to_string(), 5, false, 4, Datum::I32(0)),
        ColumnMetadata::new("c6".to_string(), 6, false, 4, Datum::I32(0)),
    ];
    let relation_md = RelationMetadata::new(
        "t1".to_string(),
        column_md,
        relation_stats_id,
        vec![IndexInfo::new(index_md_id)],
    );
    let boxed_relation_md = Box::new(relation_md.clone()) as Box<dyn Metadata>;

    // column stats
    let buckets = vec![
        Bucket::new(Datum::I32(0), Datum::I32(1), 1, 2),
        Bucket::new(Datum::I32(1), Datum::I32(3), 3, 3),
    ];
    let histogram = Histogram::new(buckets);
    let column_stats = ColumnStats::new(1, 'x'.to_string(), Datum::I32(0), Datum::I32(1), 0, Some(histogram));
    let boxed_column_stats = Box::new(column_stats.clone()) as Box<dyn Metadata>;

    // metadata cache
    let mut md_cache = MdCache::new();
    md_cache.insert(relation_stats_id, boxed_relation_stats);
    md_cache.insert(relation_md_id, boxed_relation_md);
    md_cache.insert(column_stats_id, boxed_column_stats);
    md_cache.insert(index_md_id, boxed_index_md);

    md_cache
}

fn metadata_accessor() -> MdAccessor {
    let md_cache = md_cache();
    let md_provider = Rc::new(CachedMdProvider::new(md_cache));
    MdAccessor::new(md_provider)
}

fn expected_physical_plan_with_index() -> PhysicalPlan {
    let mdid = 2;
    let table_desc = TableDesc::new(mdid);
    let index_desc = IndexDesc::new(
        4,
        "IDX_1".to_string(),
        IndexType::Btree,
        vec![ColumnVar::new(0)],
        vec![ColumnVar::new(0), ColumnVar::new(1), ColumnVar::new(2)],
    );
    let output_columns = vec![ColumnVar::new(0), ColumnVar::new(1), ColumnVar::new(2)];
    let predicate = IsNull::new(Box::new(ColumnVar::new(0)));
    let scan = PhysicalIndexScan::new(
        index_desc,
        table_desc,
        output_columns,
        Rc::new(And::new(vec![Rc::new(predicate)])),
    );
    let scan = PhysicalPlan::new(Rc::new(scan), vec![]);

    let project = vec![
        Rc::new(ColumnVar::new(1)) as Rc<dyn ScalarExpression>,
        Rc::new(ColumnVar::new(2)) as Rc<dyn ScalarExpression>,
    ];
    let project = PhysicalProject::new(project);
    PhysicalPlan::new(Rc::new(project), vec![scan])
}

// can completely cover filter
// sql: select c2, c3 from t1 where c1 is null order by c1;
// idx: key columns(c1) included columns(c1, c2, c3)
// Project(c2, c3) -> IndexScan(c1)
#[test]
fn test_sort_project_index_scan_matched() {
    let mut optimizer = Optimizer::new(Options::default());
    let rule_set = create_rule_set();

    let scan = logical_scan();
    let filter = logical_filter(vec![scan], 0, None);
    let project = logical_project(vec![filter]);
    let required_properties = required_properties(0);
    let md_accessor = metadata_accessor();

    let physical_plan = optimizer.optimize(project, required_properties, md_accessor, rule_set);
    assert_eq!(physical_plan, expected_physical_plan_with_index());
}

fn expected_physical_plan_without_index() -> PhysicalPlan {
    let mdid = 2;
    let table_desc = TableDesc::new(mdid);
    let output_columns = vec![ColumnVar::new(0), ColumnVar::new(1), ColumnVar::new(2)];

    let scan = PhysicalScan::new(table_desc, output_columns);
    let scan = PhysicalPlan::new(Rc::new(scan), vec![]);

    let predicate = IsNull::new(Box::new(ColumnVar::new(1)));
    let filter = PhysicalFilter::new(Rc::new(predicate));
    let filter = PhysicalPlan::new(Rc::new(filter), vec![scan]);

    let project = vec![
        Rc::new(ColumnVar::new(1)) as Rc<dyn ScalarExpression>,
        Rc::new(ColumnVar::new(2)) as Rc<dyn ScalarExpression>,
    ];
    let project = PhysicalProject::new(project);
    let project = PhysicalPlan::new(Rc::new(project), vec![filter]);

    let order = OrderSpec {
        order_desc: vec![Ordering {
            key: ColumnVar::new(0),
            ascending: true,
            nulls_first: true,
        }],
    };
    let sort = PhysicalSort::new(order);
    PhysicalPlan::new(Rc::new(sort), vec![project])
}

// can not cover filter
// sql: select c2, c3 from t1 where c2 is null order by c1;
// idx: key columns(c1) included columns(c1, c2, c3)
// Sort(c1) -> project(c2, c3) -> filter(c2) -> Scan
#[test]
fn test_sort_project_index_scan_not_matched() {
    let mut optimizer = Optimizer::new(Options::default());
    let rule_set = create_rule_set();

    let scan = logical_scan();
    let filter = logical_filter(vec![scan], 1, None);
    let project = logical_project(vec![filter]);
    let required_properties = required_properties(0);
    let md_accessor = metadata_accessor();

    let physical_plan = optimizer.optimize(project, required_properties, md_accessor, rule_set);
    assert_eq!(physical_plan, expected_physical_plan_without_index());
}

fn expected_physical_plan_with_index_and_filter() -> PhysicalPlan {
    let mdid = 2;
    let table_desc = TableDesc::new(mdid);
    let index_desc = IndexDesc::new(
        4,
        "IDX_1".to_string(),
        IndexType::Btree,
        vec![ColumnVar::new(0)],
        vec![ColumnVar::new(0), ColumnVar::new(1), ColumnVar::new(2)],
    );
    let output_columns = vec![ColumnVar::new(0), ColumnVar::new(1), ColumnVar::new(2)];
    let predicate = IsNull::new(Box::new(ColumnVar::new(0)));
    let scan = PhysicalIndexScan::new(
        index_desc,
        table_desc,
        output_columns,
        Rc::new(And::new(vec![Rc::new(predicate)])),
    );
    let scan = PhysicalPlan::new(Rc::new(scan), vec![]);

    let filter = PhysicalFilter::new(Rc::new(And::new(vec![Rc::new(IsNull::new(Box::new(ColumnVar::new(
        1,
    ))))])));
    let filter = PhysicalPlan::new(Rc::new(filter), vec![scan]);

    let project = vec![
        Rc::new(ColumnVar::new(1)) as Rc<dyn ScalarExpression>,
        Rc::new(ColumnVar::new(2)) as Rc<dyn ScalarExpression>,
    ];
    let project = PhysicalProject::new(project);
    PhysicalPlan::new(Rc::new(project), vec![filter])
}

// can partly cover filter
// sql: select c2, c3 from t1 where c1 is null and c2 is null order by c1;
// idx: key columns(c1) included columns(c1, c2, c3)
// project(c2, c3) -> filter(c2) -> IndexScan(c1)
#[test]
fn test_sort_project_index_scan_partly_matched() {
    let mut optimizer = Optimizer::new(Options::default());
    let rule_set = create_rule_set();

    let scan = logical_scan();
    let filter = logical_filter(vec![scan], 0, Some(1));
    let project = logical_project(vec![filter]);
    let required_properties = required_properties(0);
    let md_accessor = metadata_accessor();

    let physical_plan = optimizer.optimize(project, required_properties, md_accessor, rule_set);
    assert_eq!(physical_plan, expected_physical_plan_with_index_and_filter());
}

#[allow(dead_code)]
fn expected_physical_plan_with_index_2() -> PhysicalPlan {
    let mdid = 2;
    let table_desc = TableDesc::new(mdid);
    let index_desc = IndexDesc::new(
        4,
        "IDX_1".to_string(),
        IndexType::Btree,
        vec![ColumnVar::new(0)],
        vec![ColumnVar::new(0), ColumnVar::new(1), ColumnVar::new(2)],
    );
    let output_columns = vec![ColumnVar::new(0), ColumnVar::new(1), ColumnVar::new(2)];
    let predicate = IsNull::new(Box::new(ColumnVar::new(0)));
    let scan = PhysicalIndexScan::new(
        index_desc,
        table_desc,
        output_columns,
        Rc::new(And::new(vec![Rc::new(predicate)])),
    );
    let scan = PhysicalPlan::new(Rc::new(scan), vec![]);

    let order = OrderSpec {
        order_desc: vec![Ordering {
            key: ColumnVar::new(1),
            ascending: true,
            nulls_first: true,
        }],
    };
    let sort = PhysicalSort::new(order);
    let sort = PhysicalPlan::new(Rc::new(sort), vec![scan]);

    let project = vec![
        Rc::new(ColumnVar::new(1)) as Rc<dyn ScalarExpression>,
        Rc::new(ColumnVar::new(2)) as Rc<dyn ScalarExpression>,
    ];
    let project = PhysicalProject::new(project);
    PhysicalPlan::new(Rc::new(project), vec![sort])
}

// can completely cover filter but need another column to order by
// sql: select c2, c3 from t1 where c1 is null order by c2;
// idx: key columns(c1) included columns(c1, c2, c3)
// valid plan:
// 1. Sort(c2) -> Project(c2, c3) -> IndexScan(c1)
// 2. Project(c2, c3) -> Sort(c2) -> IndexScan(c1) -- expected
// 3. Sort(c2) -> Project(c2, c3) -> Filter(c1) -> Scan
#[test]
fn test_sort_project_index_scan_matched_2() {
    let mut optimizer = Optimizer::new(Options::default());
    let rule_set = create_rule_set();

    let scan = logical_scan();
    let filter = logical_filter(vec![scan], 0, None);
    let project = logical_project(vec![filter]);
    let required_properties = required_properties(1);
    let md_accessor = metadata_accessor();

    let physical_plan = optimizer.optimize(project, required_properties, md_accessor, rule_set);
    assert_eq!(physical_plan, expected_physical_plan_with_index_2());
}

fn expected_physical_plan_with_index_and_filter_2() -> PhysicalPlan {
    let mdid = 2;
    let table_desc = TableDesc::new(mdid);
    let index_desc = IndexDesc::new(
        4,
        "IDX_1".to_string(),
        IndexType::Btree,
        vec![ColumnVar::new(0)],
        vec![ColumnVar::new(0), ColumnVar::new(1), ColumnVar::new(2)],
    );
    let output_columns = vec![ColumnVar::new(0), ColumnVar::new(1), ColumnVar::new(2)];
    let predicate = IsNull::new(Box::new(ColumnVar::new(0)));
    let scan = PhysicalIndexScan::new(
        index_desc,
        table_desc,
        output_columns,
        Rc::new(And::new(vec![Rc::new(predicate)])),
    );
    let scan = PhysicalPlan::new(Rc::new(scan), vec![]);

    let filter = PhysicalFilter::new(Rc::new(And::new(vec![Rc::new(IsNull::new(Box::new(ColumnVar::new(
        1,
    ))))])));
    let filter = PhysicalPlan::new(Rc::new(filter), vec![scan]);

    let project = vec![
        Rc::new(ColumnVar::new(1)) as Rc<dyn ScalarExpression>,
        Rc::new(ColumnVar::new(2)) as Rc<dyn ScalarExpression>,
    ];
    let project = PhysicalProject::new(project);
    let project = PhysicalPlan::new(Rc::new(project), vec![filter]);

    let order = OrderSpec {
        order_desc: vec![Ordering {
            key: ColumnVar::new(1),
            ascending: true,
            nulls_first: true,
        }],
    };
    let sort = PhysicalSort::new(order);
    PhysicalPlan::new(Rc::new(sort), vec![project])
}

// can partly cover filter
// sql: select c2, c3 from t1 where c1 is null and c2 is null order by c2;
// idx: key columns(c1) included columns(c1, c2, c3)
// Sort(c2) -> project(c2, c3) -> filter(c2) -> IndexScan(c1)
#[test]
fn test_sort_project_index_scan_partly_matched_2() {
    let mut optimizer = Optimizer::new(Options::default());
    let rule_set = create_rule_set();

    let scan = logical_scan();
    let filter = logical_filter(vec![scan], 0, Some(1));
    let project = logical_project(vec![filter]);
    let required_properties = required_properties(1);
    let md_accessor = metadata_accessor();

    let physical_plan = optimizer.optimize(project, required_properties, md_accessor, rule_set);
    assert_eq!(physical_plan, expected_physical_plan_with_index_and_filter_2());
}
