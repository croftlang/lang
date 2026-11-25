//! Method calls: port operations through the target's adapter, repositories,
//! and the built-in methods of numbers and options.

use dn_analysis::types::Resolved;
use dn_analysis::Type;
use dn_syntax::ast::{Argument, Expr, ExprKind};

use super::Writer;
use crate::names::c_name;

impl<'g, 'a> Writer<'g, 'a> {
    pub fn method(
        &mut self,
        receiver: &Expr,
        name: &str,
        arguments: &[Argument],
        ty: &Type,
    ) -> String {
        let receiver_type = self.type_of(receiver, None);
        match &receiver_type {
            Type::Port(port) => self.port_call(port, name, arguments, ty),
            Type::Repository(repository) => {
                let aggregate = self.gen.model.repository_aggregate(repository);
                let parameter = match (name, aggregate) {
                    ("find", Some(aggregate)) => self.gen.model.identity_of(&aggregate),
                    (_, Some(aggregate)) => Type::Named(aggregate),
                    _ => Type::Unknown,
                };
                let passed = self.pass_all(arguments, &[("value".to_string(), parameter)]);
                self.call_into(&format!("{}_{name}", c_name(repository)), passed, ty)
            }
            Type::String => {
                let (text, _) = self.eval(receiver, None);
                self.string_method(&text, name, arguments, ty)
            }
            Type::Bytes => {
                let (bytes, _) = self.eval(receiver, None);
                self.bytes_method(&bytes, name, arguments, ty)
            }
            Type::Decimal(places) if name == "to_string" => {
                let number = self.value(receiver, &Type::Decimal(*places));
                let temp = self.temp(&Type::String);
                self.line(format!(
                    "dn_decimal_to_string({number}, {places}, &{temp});"
                ));
                temp
            }
            Type::Decimal(places) if name == "round" => {
                let number = self.value(receiver, &Type::Decimal(*places));
                format!(
                    "dn_divide_even({number}, {}L)",
                    crate::types::constant::scale(*places)
                )
            }
            Type::Int if name == "to_string" => {
                let number = self.value(receiver, &Type::Int);
                let temp = self.temp(&Type::String);
                self.line(format!("dn_int_to_string({number}, &{temp});"));
                temp
            }
            Type::List(item) => {
                let (list, _) = self.eval(receiver, None);
                self.list_method(&list, item, name, arguments, ty)
            }
            Type::Option(item) => {
                let (option, _) = self.eval(receiver, None);
                self.option_method(&option, item, name, arguments, ty)
            }
            _ => "0".to_string(),
        }
    }

    /// A port operation calls the adapter this target binds to the port.
    fn port_call(
        &mut self,
        port: &Resolved,
        name: &str,
        arguments: &[Argument],
        ty: &Type,
    ) -> String {
        if port.context.is_empty() {
            return self.builtin_port_call(&port.name, arguments, ty);
        }
        let Some(adapter) = self.gen.bindings.get(port).cloned() else {
            return "0".to_string();
        };
        let parameters = self.gen.operation_parameters(port, name);
        let passed = self.pass_all(arguments, &parameters);
        self.call_into(&format!("{}_{name}", c_name(&adapter)), passed, ty)
    }

    /// An operation on a built-in port, which the runtime supplies on every target.
    fn builtin_port_call(&mut self, port: &str, arguments: &[Argument], ty: &Type) -> String {
        if port == "Console" {
            let passed = self.pass_all(arguments, &[("text".to_string(), Type::String)]);
            return self.call_into("dn_console_print", passed, ty);
        }
        self.all_arguments(ty)
    }

    /// `arguments.all()`: the program's command-line arguments.
    fn all_arguments(&mut self, ty: &Type) -> String {
        let list = self.temp(ty);
        let index = self.variable("index", &Type::Int);
        self.line(format!("{list}.length = 0;"));
        let capacity = self.gen.sizes.list_capacity(&Type::String);
        self.open(format!(
            "for ({index} = 0; {index} < dn_argument_count() && {index} < {capacity}; {index}++)"
        ));
        self.line(format!(
            "dn_argument({index}, &{list}.items[{list}.length]);"
        ));
        self.line(format!("{list}.length++;"));
        self.close();
        list
    }

    /// Calls the lambda argument with `item`, giving a value of type `result`.
    pub fn lambda(
        &mut self,
        arguments: &[Argument],
        item: String,
        item_type: &Type,
        result: &Type,
    ) -> String {
        let Some(ExprKind::Lambda(parameters, body)) = arguments.first().map(|a| &a.value.kind)
        else {
            return "0".to_string();
        };
        self.push_scope();
        if let Some(parameter) = parameters.first() {
            self.bind(&parameter.name, item, item_type.clone());
        }
        let value = self.value(body, result);
        self.pop_scope();
        value
    }
}
