//! Regenerate the entity fixture golden:
//!   cargo run -p core-entity --example dump_entity_fixtures > \
//!     harness/fixtures/entities/families.golden

fn main() {
    print!("{}", core_entity::fixtures::dump_all_families());
}
