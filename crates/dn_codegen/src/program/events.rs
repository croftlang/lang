//! Events and policies. Commands queue their events; once a service ends,
//! the queue is handed to the policies that react to each event.

use dn_analysis::symbols::SymbolKind;
use dn_analysis::Type;

use crate::function::{Returns, Writer};
use crate::names::{c_name, event_kind};
use crate::Generator;

const EVENT_CAPACITY: usize = 16;

impl<'a> Generator<'a> {
    /// `dn_event`, which holds any one event, for the header.
    pub fn event_definition(&self) -> String {
        let mut out = String::new();
        let mut cases = String::new();
        for (index, event) in self.model.all(SymbolKind::Event).iter().enumerate() {
            out.push_str(&format!("#define {} {}\n", event_kind(event), index + 1));
            let c_type = self.c_type(&Type::Named(event.clone()));
            cases.push_str(&format!("        {c_type} {};\n", c_name(event)));
        }
        let body =
            format!("    dn_u16 kind;\n    union {{\n{cases}        dn_u8 none;\n    }} as;\n");
        out.push('\n');
        out.push_str(&crate::types::define::struct_definition("dn_event", &body));
        out.push_str("dn_u16 dn_events_mark(void);\nvoid dn_events_rollback(dn_u16 mark);\n");
        out.push_str(
            "void dn_events_push(const dn_event *event);\nvoid dn_events_dispatch(void);\n\n",
        );
        out
    }

    /// The policies, the event queue, and the function that hands events out.
    pub fn events(&self, prototypes: &mut Vec<String>) -> String {
        let mut out = String::new();
        let mut handlers = String::new();
        for found in self.model.all(SymbolKind::Policy) {
            let context = &self.program.contexts[&found.context];
            let Some(policy) = context.policies.iter().find(|p| p.name.name == found.name) else {
                continue;
            };
            let Some(event) = self.model.resolve(&found.context, None, &policy.event) else {
                continue;
            };
            let event_type = Type::Named(event.clone());
            let signature = format!(
                "void {}(const {} *p_event)",
                c_name(&found),
                self.c_type(&event_type)
            );
            prototypes.push(format!("{signature};"));
            let mut writer = Writer::new(self, &found.context, Returns::Nothing);
            writer.parameter("p_event");
            writer.bind("event", "(*p_event)".to_string(), event_type);
            writer.value(&policy.body, &Type::Unit);
            out.push_str(&writer.finish(&signature));
            handlers.push_str(&format!(
                "    if (event->kind == {}) {}(&event->as.{});\n",
                event_kind(&event),
                c_name(&found),
                c_name(&event)
            ));
        }
        out.push_str(&format!(
            "dn_event dn_events[{EVENT_CAPACITY}];\ndn_u16 dn_event_count = 0;\n\n\
             dn_u16 dn_events_mark(void)\n{{\n    return dn_event_count;\n}}\n\n\
             void dn_events_rollback(dn_u16 mark)\n{{\n    dn_event_count = mark;\n}}\n\n\
             void dn_events_push(const dn_event *event)\n{{\n    DN_CHECK(dn_event_count < {EVENT_CAPACITY}, \"too many events at once\");\n    \
             if (dn_event_count < {EVENT_CAPACITY}) {{\n        dn_events[dn_event_count] = *event;\n        dn_event_count++;\n    }}\n}}\n\n\
             static void dn_events_handle(const dn_event *event)\n{{\n{handlers}    (void) event;\n}}\n\n\
             void dn_events_dispatch(void)\n{{\n    dn_event event;\n    dn_u16 index;\n\n    while (dn_event_count > 0) {{\n        \
             event = dn_events[0];\n        for (index = 1; index < dn_event_count; index++) {{\n            dn_events[index - 1] = dn_events[index];\n        }}\n        \
             dn_event_count--;\n        dn_events_handle(&event);\n    }}\n}}\n\n"
        ));
        out
    }
}
