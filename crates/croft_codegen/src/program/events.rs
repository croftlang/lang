//! Events and policies. Commands queue their events; saving the aggregate
//! that raised them keeps them; once a service ends, the kept ones are handed
//! to the policies that react to each event.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::Type;

use crate::function::{Returns, Writer};
use crate::names::{aggregate_kind, c_name, event_kind};
use crate::Generator;

const EVENT_CAPACITY: usize = 16;
const EVENT_QUEUE: &str = include_str!("../runtime/events.c");

impl<'a> Generator<'a> {
    /// `croft_event`, which holds any one event, for the header.
    pub fn event_definition(&self) -> String {
        let mut out = String::new();
        let mut cases = String::new();
        for (index, event) in self.model.all(SymbolKind::Event).iter().enumerate() {
            out.push_str(&format!("#define {} {}\n", event_kind(event), index + 1));
            let c_type = self.c_type(&Type::Named(event.clone()));
            cases.push_str(&format!("        {c_type} {};\n", c_name(event)));
        }
        for (index, aggregate) in self.model.all(SymbolKind::Aggregate).iter().enumerate() {
            out.push_str(&format!(
                "#define {} {}\n",
                aggregate_kind(aggregate),
                index + 1
            ));
        }
        let body = format!(
            "    croft_u16 kind;\n    croft_u16 aggregate;\n    croft_int key;\n    croft_bool kept;\n    \
             union {{\n{cases}        croft_u8 none;\n    }} as;\n"
        );
        out.push('\n');
        out.push_str(&crate::types::define::struct_definition(
            "croft_event",
            &body,
        ));
        out.push_str(
            "croft_u16 croft_events_mark(void);\nvoid croft_events_rollback(croft_u16 mark);\n",
        );
        out.push_str("void croft_events_push(const croft_event *event);\n");
        out.push_str("void croft_events_keep(croft_u16 aggregate, croft_int key);\n");
        out.push_str("void croft_events_dispatch_from(croft_u16 mark);\n\n");
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
            "#define CROFT_EVENT_CAPACITY {EVENT_CAPACITY}\n\n"
        ));
        out.push_str(&EVENT_QUEUE.replace("/* HANDLERS */", &handlers));
        out.push('\n');
        out
    }
}
