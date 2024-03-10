use crate::cost::{COST_INDEX_FILTER_COST_UNIT, COST_INDEX_SCAN_TUP_COST_UNIT, COST_INDEX_SCAN_TUP_RANDOM_FACTOR};
use crate::expression::ColumnVar;
use crate::operator::logical_index_scan::IndexDesc;
use crate::operator::logical_scan::TableDesc;
use crate::operator::{OperatorId, PhysicalOperator};
use crate::property::PhysicalProperties;
use crate::{Demo, GroupRef};
use cso_core::cost::Cost;
use cso_core::expression::ScalarExpression;
use cso_core::metadata::Stats;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct PhysicalIndexScan {
    index_desc: IndexDesc,
    table_desc: TableDesc,
    output_columns: Vec<ColumnVar>,
    _predicate: Rc<dyn ScalarExpression>,
}

impl PhysicalIndexScan {
    pub fn new(
        index_desc: IndexDesc,
        table_desc: TableDesc,
        output_columns: Vec<ColumnVar>,
        _predicate: Rc<dyn ScalarExpression>,
    ) -> Self {
        PhysicalIndexScan {
            index_desc,
            table_desc,
            output_columns,
            _predicate,
        }
    }
}

impl cso_core::operator::PhysicalOperator<Demo> for PhysicalIndexScan {
    fn name(&self) -> &str {
        "physical index scan"
    }

    fn operator_id(&self) -> &OperatorId {
        &OperatorId::PhysicalIndexScan
    }

    fn derive_output_properties(&self, _: &[Rc<PhysicalProperties>]) -> Rc<PhysicalProperties> {
        // todo
        Rc::new(PhysicalProperties::new())
    }

    fn required_properties(&self, _input_prop: Rc<PhysicalProperties>) -> Vec<Vec<Rc<PhysicalProperties>>> {
        vec![vec![]]
    }

    fn compute_cost(&self, inputs: &[GroupRef], stats: Option<&dyn Stats>) -> Cost {
        debug_assert!(stats.is_some());
        debug_assert!(inputs.is_empty());

        let index_key_column_count = self.index_desc.key_columns_count() as f64;
        let cost_per_index_row = index_key_column_count * COST_INDEX_FILTER_COST_UNIT + COST_INDEX_SCAN_TUP_COST_UNIT;
        let row_count = stats.unwrap().output_row_count() as f64;
        let cost = row_count * cost_per_index_row + COST_INDEX_SCAN_TUP_RANDOM_FACTOR;
        Cost::new(cost)
    }

    fn equal(&self, other: &PhysicalOperator) -> bool {
        match other.downcast_ref::<PhysicalIndexScan>() {
            Some(other) => self.eq(other),
            None => false,
        }
    }
}

impl PartialEq for PhysicalIndexScan {
    fn eq(&self, other: &Self) -> bool {
        let predicate_equal = {
            // if self.predicate.len() != other.predicate.len() {
            //     return false;
            // }
            //
            // for (elem1, elem2) in self.predicate.iter().zip(other.predicate.as_ref()) {
            //     if elem1 != elem2 {
            //         return false;
            //     }
            // }
            true
        };
        self.index_desc == other.index_desc
            && self.table_desc == other.table_desc
            && self.output_columns == other.output_columns
            && predicate_equal
    }
}
