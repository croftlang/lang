//! Repositories. For now each keeps its aggregates in memory; saving through
//! the storage port comes later. Saving an aggregate keeps the events its
//! commands raised, so policies hear about them.

use dn_analysis::symbols::SymbolKind;
use dn_analysis::Type;

use crate::names::{aggregate_kind, c_name, error_code};
use crate::Generator;

const REPOSITORY_CAPACITY: usize = 8;

impl<'a> Generator<'a> {
    pub fn repositories(&self, prototypes: &mut Vec<String>) -> String {
        let mut out = String::new();
        for repository in self.model.all(SymbolKind::Repository) {
            let Some(aggregate) = self.model.repository_aggregate(&repository) else {
                continue;
            };
            let name = c_name(&repository);
            let aggregate_type = Type::Named(aggregate.clone());
            let item = self.c_type(&aggregate_type);
            let identity = self.model.identity_of(&aggregate);
            let identity_type = self.c_type(&identity);
            let field = self.identity_field(&aggregate);
            let found = self.c_type(&Type::result(aggregate_type, Type::Errors(Vec::new())));
            let saved = self.c_type(&Type::result(Type::Unit, Type::Errors(Vec::new())));
            let not_found = self
                .model
                .lookup(&aggregate.context, &format!("{}.NotFound", aggregate.name));
            let not_found = not_found.map_or("0".to_string(), |error| error_code(&error));
            let kind = aggregate_kind(&aggregate);
            let key = self.event_key(&aggregate, &format!("aggregate->{field}"));
            let find = format!("void {name}_find({identity_type} identity, {found} *out)");
            let save = format!("void {name}_save(const {item} *aggregate, {saved} *out)");
            prototypes.push(format!("{find};"));
            prototypes.push(format!("{save};"));
            out.push_str(&format!(
                "{item} {name}_items[{REPOSITORY_CAPACITY}];\ndn_u16 {name}_count = 0;\n\n\
                 {find}\n{{\n    dn_u16 index;\n\n    for (index = 0; index < {name}_count; index++) {{\n        \
                 if ({name}_items[index].{field} == identity) {{\n            out->ok = 1;\n            out->value = {name}_items[index];\n            return;\n        }}\n    }}\n    \
                 out->ok = 0;\n    out->error.code = {not_found};\n}}\n\n\
                 {save}\n{{\n    dn_u16 index;\n\n    out->ok = 1;\n    out->value = 0;\n    dn_events_keep({kind}, {key});\n    for (index = 0; index < {name}_count; index++) {{\n        \
                 if ({name}_items[index].{field} == aggregate->{field}) {{\n            {name}_items[index] = *aggregate;\n            return;\n        }}\n    }}\n    \
                 DN_CHECK({name}_count < {REPOSITORY_CAPACITY}, \"a repository is full\");\n    \
                 if ({name}_count < {REPOSITORY_CAPACITY}) {{\n        {name}_items[{name}_count] = *aggregate;\n        {name}_count++;\n    }}\n}}\n\n"
            ));
        }
        out
    }
}
