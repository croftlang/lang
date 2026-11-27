//! The state shared by everything generated for one program and target.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::{Facts, Model, Program, Type};

use crate::types::sizes::Sizes;

/// Generates the C for one program on one target.
pub struct Generator<'a> {
    pub(crate) program: &'a Program,
    pub(crate) model: Model<'a>,
    pub(crate) facts: &'a Facts,
    pub(crate) target: String,
    /// Which adapter fills each port on this target.
    pub(crate) bindings: HashMap<Resolved, Resolved>,
    /// The adapter for `Storage` that keeps each repository's aggregates.
    pub(crate) storages: HashMap<Resolved, Resolved>,
    /// Types that need functions to turn them into bytes and back.
    pub(crate) serialized: RefCell<(Vec<Type>, HashSet<Type>)>,
    /// What runs first: the domain's body, or the target's `main` service.
    pub(crate) start: Resolved,
    pub(crate) sizes: Sizes,
    /// Every C type used, in the order first seen; defined later in dependency order.
    pub(crate) used_types: RefCell<(Vec<Type>, HashSet<Type>)>,
    /// Types that need an equality function.
    pub(crate) equalities: RefCell<(Vec<Type>, HashSet<Type>)>,
    /// Sort functions asked for: each one's name, C item type, and test that
    /// one item comes before another.
    pub(crate) sorts: RefCell<Vec<(String, String, String)>>,
    /// Debug-build constraint checks: each one's prototype and definition.
    pub(crate) checks: RefCell<Vec<(String, String)>>,
    /// Types whose field-constraint check has been generated.
    pub(crate) checked_types: RefCell<HashSet<Resolved>>,
    /// The capacities of the `croft_text_N` structs string fields are kept in.
    pub(crate) texts: RefCell<std::collections::BTreeSet<i64>>,
    /// The sizes of `croft_bytes_N` the program uses, as `texts` for strings.
    pub(crate) byte_sizes: RefCell<std::collections::BTreeSet<i64>>,
    /// Native C functions called by adapters, with their prototypes.
    pub(crate) natives: RefCell<Vec<(String, String)>>,
    /// What each function and translation gives back, with the bounds of its
    /// strings worked out from its body. One that isn't here gives back
    /// its declared type.
    pub(crate) results: RefCell<HashMap<Resolved, Type>>,
}

impl<'a> Generator<'a> {
    /// Reads the target's port bindings from `croft.toml` and finds what runs first.
    pub fn new(program: &'a Program, target: &str) -> Result<Generator<'a>, String> {
        let model = Model::new(program);

        let Some(manifest) = &program.manifest else {
            return Err(
                "the program has no croft.toml, so there is nothing to compile".to_string(),
            );
        };

        let Some(target_config) = manifest.target(target) else {
            let names: Vec<String> = manifest
                .targets
                .iter()
                .map(|t| format!("`{}`", t.name.name))
                .collect();

            return Err(format!(
                "croft.toml has no target `{target}`; it has {}",
                names.join(", ")
            ));
        };

        let mut bindings = HashMap::new();
        let mut storages = HashMap::new();

        for (port, adapter) in &target_config.adapters {
            if let Some(repository) = find(&model, &port.name, SymbolKind::Repository) {
                if let Some(adapter) = find(&model, &adapter.name, SymbolKind::Adapter) {
                    storages.insert(repository, adapter);
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

        if let Some(memory) = model.lookup("Std.Storage", "MemoryStorage") {
            for repository in model.all(SymbolKind::Repository) {
                storages.entry(repository).or_insert_with(|| memory.clone());
            }
        }

        let sizes = Sizes::measure(program, &model);

        let mut generator = Generator {
            program,
            model,
            facts: &program.facts,
            target: target.to_string(),
            bindings,
            storages,
            serialized: RefCell::default(),
            start,
            sizes,
            used_types: RefCell::default(),
            equalities: RefCell::default(),
            sorts: RefCell::default(),
            checks: RefCell::default(),
            checked_types: RefCell::default(),
            texts: RefCell::default(),
            byte_sizes: RefCell::default(),
            natives: RefCell::default(),
            results: RefCell::default(),
        };

        generator.make_room_for_storage();

        Ok(generator)
    }

    /// Makes every `Bytes` big enough to hold a whole saved aggregate, and
    /// every string a file name such as `rooms12.dat`.
    fn make_room_for_storage(&mut self) {
        let mut largest = 0;

        for repository in self.storages.keys() {
            if let Some(aggregate) = self.model.repository_aggregate(repository) {
                largest = largest.max(self.saved_size(&aggregate));
            }
        }

        if !self.storages.is_empty() {
            self.sizes.bytes_capacity = self.sizes.bytes_capacity.max(largest);
            self.sizes.string_capacity = self.sizes.string_capacity.max(12);
        }
    }
}

/// What writing functions adds to a `Generator`, saved so a round of
/// `settle_results` leaves no trace.
pub(crate) struct SavedState {
    serialized: (Vec<Type>, HashSet<Type>),
    used_types: (Vec<Type>, HashSet<Type>),
    equalities: (Vec<Type>, HashSet<Type>),
    sorts: Vec<(String, String, String)>,
    checks: Vec<(String, String)>,
    checked_types: HashSet<Resolved>,
    texts: std::collections::BTreeSet<i64>,
    byte_sizes: std::collections::BTreeSet<i64>,
    natives: Vec<(String, String)>,
}

impl<'a> Generator<'a> {
    /// Copies what writing functions has added so far.
    pub(crate) fn save_state(&self) -> SavedState {
        SavedState {
            serialized: self.serialized.borrow().clone(),
            used_types: self.used_types.borrow().clone(),
            equalities: self.equalities.borrow().clone(),
            sorts: self.sorts.borrow().clone(),
            checks: self.checks.borrow().clone(),
            checked_types: self.checked_types.borrow().clone(),
            texts: self.texts.borrow().clone(),
            byte_sizes: self.byte_sizes.borrow().clone(),
            natives: self.natives.borrow().clone(),
        }
    }

    /// Puts back what `save_state` copied.
    pub(crate) fn restore_state(&self, saved: &SavedState) {
        *self.serialized.borrow_mut() = saved.serialized.clone();
        *self.used_types.borrow_mut() = saved.used_types.clone();
        *self.equalities.borrow_mut() = saved.equalities.clone();
        *self.sorts.borrow_mut() = saved.sorts.clone();
        *self.checks.borrow_mut() = saved.checks.clone();
        *self.checked_types.borrow_mut() = saved.checked_types.clone();
        *self.texts.borrow_mut() = saved.texts.clone();
        *self.byte_sizes.borrow_mut() = saved.byte_sizes.clone();
        *self.natives.borrow_mut() = saved.natives.clone();
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
