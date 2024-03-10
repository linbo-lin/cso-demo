use crate::cost::{cost_children, COST_FILTER_COL_COST_UNIT};
use crate::operator::{OperatorId, PhysicalOperator};
use crate::property::PhysicalProperties;
use crate::{Demo, GroupRef};
use cso_core::cost::Cost;
use cso_core::expression::ScalarExpression;
use cso_core::metadata::Stats;
use cso_core::ColumnRefSet;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct PhysicalFilter {
    predicate: Rc<dyn ScalarExpression>,
}

impl PhysicalFilter {
    pub fn new(predicate: Rc<dyn ScalarExpression>) -> Self {
        assert!(predicate.is_boolean_expression());
        PhysicalFilter { predicate }
    }

    pub fn predicate(&self) -> &dyn ScalarExpression {
        self.predicate.as_ref()
    }
}

impl cso_core::operator::PhysicalOperator<Demo> for PhysicalFilter {
    fn name(&self) -> &str {
        "physical filter"
    }

    fn operator_id(&self) -> &OperatorId {
        &OperatorId::PhysicalFilter
    }

    fn derive_output_properties(&self, child_output_props: &[Rc<PhysicalProperties>]) -> Rc<PhysicalProperties> {
        child_output_props[0].clone()
    }

    fn required_properties(&self, input_prop: Rc<PhysicalProperties>) -> Vec<Vec<Rc<PhysicalProperties>>> {
        vec![vec![input_prop], vec![Rc::new(PhysicalProperties::new())]]
    }

    fn compute_cost(&self, inputs: &[GroupRef], stats: Option<&dyn Stats>) -> Cost {
        debug_assert!(stats.is_some());

        let mut filter_columns = ColumnRefSet::new();
        self.predicate.derive_used_columns(&mut filter_columns);
        let filter_columns_count = filter_columns.len() as f64;

        let row_count = stats.unwrap().output_row_count() as f64;
        let cost_local = row_count * filter_columns_count * COST_FILTER_COL_COST_UNIT;
        let cost_child = cost_children(inputs).value();
        Cost::new(cost_local + cost_child)
    }

    fn equal(&self, other: &PhysicalOperator) -> bool {
        match other.downcast_ref::<PhysicalFilter>() {
            Some(other) => self.eq(other),
            None => false,
        }
    }
}

impl PartialEq for PhysicalFilter {
    fn eq(&self, other: &Self) -> bool {
        self.predicate.equal(other.predicate())
    }
}
