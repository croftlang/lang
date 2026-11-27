//! Method calls: port operations through the target's adapter, repository
//! operations, and the built-in methods of every other type.

use croft_analysis::standard::CONSOLE_CONTEXT;
use croft_analysis::types::Resolved;
use croft_analysis::Type;
use croft_syntax::ast::{Argument, Expr, ExprKind};

use super::{Into, Writer};
use crate::names::c_name;

/// The most characters a whole number takes as text: `-2147483648`.
const INT_DIGITS: i64 = 11;

impl<'g, 'a> Writer<'g, 'a> {
    /// `receiver.name(arguments)`, chosen by the type of `receiver`.
    pub fn method(
        &mut self,
        receiver: &Expr,
        name: &str,
        arguments: &[Argument],
        ty: &Type,
        into: Into<'_>,
    ) -> String {
        let receiver_type = self.type_of(receiver, None);

        match &receiver_type {
            Type::Port(port) => self.port_call(port, name, arguments, ty, into),
            Type::Repository(repository) => {
                let parameters = self.gen.operation_parameters(repository, name);
                let passed = self.pass_all(arguments, &parameters);
                let function = format!("{}_{name}", c_name(repository));
                self.call_to(&function, passed, ty, into)
            }
            Type::String => {
                let (text, bounded) = self.bounded_view(receiver);
                self.string_method((&text, &bounded), name, arguments, ty)
            }
            Type::Map(key, value) => {
                let map = self.value(receiver, &receiver_type);
                self.map_method(&map, key, value, name, arguments, ty)
            }
            Type::Bytes => {
                let (bytes, bounded) = self.bytes_place(receiver);
                self.bytes_method((&bytes, &bounded), name, arguments, ty)
            }
            Type::Decimal(places) if name == "to_string" => {
                let number = self.value(receiver, &Type::Decimal(*places));
                let temp = self.temp(&Type::String);

                self.line(format!(
                    "croft_decimal_to_string({number}, {places}, CROFT_OUT({temp}));"
                ));

                temp
            }
            Type::Decimal(places) if name == "round" => {
                let number = self.value(receiver, &Type::Decimal(*places));

                format!(
                    "croft_divide_even({number}, {}L)",
                    crate::types::constant::scale(*places)
                )
            }
            Type::Int if name == "to_string" => {
                let number = self.value(receiver, &Type::Int);
                let digits = self.gen.text_type(INT_DIGITS);
                let temp = self.temp(&digits);
                self.line(format!("croft_int_to_string({number}, CROFT_OUT({temp}));"));
                temp
            }
            Type::List(_) => {
                let (list, bounded) = self.eval(receiver, None);
                self.list_method(&list, &bounded, name, arguments, ty)
            }
            Type::Option(_) => {
                let (option, bounded) = self.eval(receiver, None);
                let Type::Option(item) = bounded else {
                    return "0".to_string();
                };
                self.option_method(&option, &item, name, arguments, ty)
            }
            Type::Result(..) => {
                let (result, bounded) = self.eval(receiver, None);
                let Type::Result(value, error) = bounded else {
                    return "0".to_string();
                };
                self.result_method(&result, (&value, &error), name, arguments, ty)
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
        into: Into<'_>,
    ) -> String {
        // Every target supplies `Std.Console`'s ports, so no adapter is bound.
        if port.context == CONSOLE_CONTEXT {
            return self.builtin_port_call(&port.name, name, arguments, ty);
        }

        let Some(adapter) = self.gen.bindings.get(port).cloned() else {
            return "0".to_string();
        };

        let parameters = self.gen.operation_parameters(port, name);
        let passed = self.pass_all(arguments, &parameters);
        let result = self.gen.operation_result(port, name);
        let result = self.gen.keep_bound(&result, ty);

        self.call_to(
            &format!("{}_{name}", c_name(&adapter)),
            passed,
            &result,
            into,
        )
    }

    /// An operation of `Console`, `Arguments`, `Environment` or `Process`,
    /// which the runtime supplies on every target.
    fn builtin_port_call(
        &mut self,
        port: &str,
        name: &str,
        arguments: &[Argument],
        ty: &Type,
    ) -> String {
        let text = [("text".to_string(), Type::String)];

        match (port, name) {
            ("Console", "read_line") => self.read_line(ty),
            ("Console", "print_error") => {
                let passed = self.pass_all(arguments, &text);
                self.call_into("croft_console_print_error", passed, ty)
            }
            ("Console", _) => {
                let passed = self.pass_all(arguments, &text);
                self.call_into("croft_console_print", passed, ty)
            }
            ("Environment", _) => self.environment_get(arguments, ty),
            ("Process", _) => {
                let passed = self.pass_all(arguments, &[("code".to_string(), Type::Int)]);
                self.call_into("croft_process_exit", passed, ty)
            }
            _ => self.all_arguments(ty),
        }
    }

    /// `Console.read_line()`: the next line of input, or None at its end.
    fn read_line(&mut self, ty: &Type) -> String {
        let line = self.temp(ty);

        self.line(format!(
            "croft_console_read_line(&{line}.some, CROFT_OUT({line}.value));"
        ));

        line
    }

    /// `Environment.get(name)`: the variable's value, or None when it isn't set.
    fn environment_get(&mut self, arguments: &[Argument], ty: &Type) -> String {
        let passed = self.pass_all(arguments, &[("name".to_string(), Type::String)]);
        let name = passed.first().cloned().unwrap_or_default();
        let value = self.temp(ty);

        self.line(format!(
            "croft_environment_get({name}, &{value}.some, CROFT_OUT({value}.value));"
        ));

        value
    }

    /// `Arguments.all()`: the program's command-line arguments. With checks
    /// on, more of them than the list holds stops the program.
    fn all_arguments(&mut self, ty: &Type) -> String {
        let list = self.temp(ty);
        let index = self.index_variable();

        self.line(format!("{list}.length = 0;"));

        let capacity = self.gen.capacity(ty);

        self.line(format!(
            "CROFT_CHECK(croft_argument_count() <= {capacity}, \"there are more arguments than a list can hold\");"
        ));

        self.open(format!(
            "for ({index} = 0; {index} < croft_argument_count() && {index} < {capacity}; {index}++)"
        ));

        self.line(format!(
            "croft_argument({index}, CROFT_OUT({list}.items[{list}.length]));"
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
