//! The state shared by everything generated for one program and target.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use dn_analysis::symbols::SymbolKind;
use dn_analysis::types::Resolved;
use dn_analysis::{Facts, Model, Program, Type};

use crate::types::sizes::Sizes;

pub struct Generator<'a> {
    pub(crate) program: &'a Program,
    pub(crate) model: Model<'a>,
    pub(crate) facts: &'a Facts,
    pub(crate) target: String,
    /// Which adapter fills each port on this target.
    pub(crate) bindings: HashMap<Resolved, Resolved>,
    /// Repositories this target keeps in `FileStorage`; the rest stay in memory.
    pub(crate) file_storage: HashSet<Resolved>,
    /// Types that need functions to turn them into bytes and back.
    pub(crate) serialized: RefCell<(Vec<Type>, HashSet<Type>)>,
    /// What runs first: the domain's body, or the target's `main` service.
    pub(crate) start: Resolved,
    pub(crate) sizes: Sizes,
    /// Every C type used, in the order first seen; defined later in dependency order.
    pub(crate) used_types: RefCell<(Vec<Type>, HashSet<Type>)>,
    /// Types that need an equality function.
    pub(crate) equalities: RefCell<(Vec<Type>, HashSet<Type>)>,
    /// Debug-build constraint checks: each one's prototype and definition.
    pub(crate) checks: RefCell<Vec<(String, String)>>,
    /// Types whose field-constraint check has been generated.
    pub(crate) checked_types: RefCell<HashSet<Resolved>>,
    /// Native C functions called by adapters, with their prototypes.
    pub(crate) natives: RefCell<Vec<(String, String)>>,
}

impl<'a> Generator<'a> {
    pub fn new(program: &'a Program, target: &str) -> Result<Generator<'a>, String> {
        let model = Model::new(program);
        let Some(manifest) = &program.manifest else {
            return Err("the program has no dn.toml, so there is nothing to compile".to_string());
        };
        let Some(target_config) = manifest.target(target) else {
            let names: Vec<String> = manifest
                .targets
                .iter()
                .map(|t| format!("`{}`", t.name.name))
                .collect();
            return Err(format!(
                "dn.toml has no target `{target}`; it has {}",
                names.join(", ")
            ));
        };
        let mut bindings = HashMap::new();
        let mut file_storage = HashSet::new();
        for (port, adapter) in &target_config.adapters {
            if let Some(repository) = find(&model, &port.name, SymbolKind::Repository) {
                if adapter.name == "FileStorage" {
                    file_storage.insert(repository);
                }
                continue;
            }
            let port = find(&model, &port.name, SymbolKind::Port);
            let adapter = find(&model, &adapter.name, SymbolKind::Adapter);
            if let (Some(port), Some(adapter)) = (port, adapter) {
                bindings.insert(port, adapter);
            }
        }
        let start = match (&target_config.main, program.domains.first()) {
            (Some(main), _) => find(&model, &main.name, SymbolKind::Service),
            (None, Some(domain)) => model.lookup(&domain.name.name, &domain.name.name),
            (None, None) => None,
        };
        let start = start
            .ok_or_else(|| format!("target `{target}` has nothing to run: declare a `domain`"))?;
        let sizes = Sizes::measure(program, &model);
        let mut generator = Generator {
            program,
            model,
            facts: &program.facts,
            target: target.to_string(),
            bindings,
            file_storage,
            serialized: RefCell::default(),
            start,
            sizes,
            used_types: RefCell::default(),
            equalities: RefCell::default(),
            checks: RefCell::default(),
            checked_types: RefCell::default(),
            natives: RefCell::default(),
        };
        generator.make_room_for_files();
        Ok(generator)
    }

    /// With `FileStorage`, every `Bytes` must hold a whole saved aggregate,
    /// and every string a file name such as `rooms12.dat`.
    fn make_room_for_files(&mut self) {
        let mut largest = 0;
        for repository in &self.file_storage {
            if let Some(aggregate) = self.model.repository_aggregate(repository) {
                largest = largest.max(4 + self.serialized_size(&Type::Named(aggregate)));
            }
        }
        if !self.file_storage.is_empty() {
            self.sizes.bytes_capacity = self.sizes.bytes_capacity.max(largest);
            self.sizes.string_capacity = self.sizes.string_capacity.max(12);
        }
    }
}

/// Finds a declaration by name in any context; `Dungeon.PlayGame` names the context.
fn find(model: &Model, name: &str, kind: SymbolKind) -> Option<Resolved> {
    let all = model.all(kind);
    if let Some((context, rest)) = name.split_once('.') {
        if let Some(found) = all.iter().find(|r| r.context == context && r.name == rest) {
            return Some(found.clone());
        }
    }
    all.into_iter().find(|r| r.name == name)
}
