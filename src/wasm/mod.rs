pub mod call_conv;
mod defines;
mod imports;
pub mod types;

pub use self::types::FunctionType;
pub use self::types::ValueType;
use self::types::{GlobalType, Type};
pub use parity_wasm::elements::BlockType;
pub use parity_wasm::elements::BrTableData;
pub use parity_wasm::elements::Instruction;
pub use parity_wasm::elements::Instructions;
use std::ops::Index;

pub trait Entry<T: Type> {
    fn get_type(&self) -> &T;
}

pub trait Def<T: Type>: Entry<T> {}

#[derive(Debug)]
pub struct Function {
    ty: FunctionType,
    code: Instructions,
}

impl Entry<FunctionType> for Function {
    fn get_type(&self) -> &FunctionType {
        &self.ty
    }
}
impl Def<FunctionType> for Function {}

impl Function {
    pub fn new(
        func_types: &Vec<FunctionType>,
        func_def: parity_wasm::elements::Func,
        func_body: parity_wasm::elements::FuncBody,
    ) -> Self {
        Self {
            ty: func_types[func_def.type_ref() as usize].clone(),
            code: func_body.code().clone(),
        }
    }

    pub fn instructions(&self) -> &[Instruction] {
        self.code.elements()
    }
}

#[derive(Debug)]
struct Import<T: types::Type>(T);

impl<T: Type> Entry<T> for Import<T> {
    fn get_type(&self) -> &T {
        &self.0
    }
}

#[derive(Debug)]
pub struct CombinedDeclear<T: Def<U>, U: Type> {
    defines: Vec<T>,
    imports: Vec<Import<U>>,
}

impl<T: Def<U>, U: Type> CombinedDeclear<T, U> {
    pub fn len(&self) -> usize {
        self.defines.len() + self.imports.len()
    }

    pub fn get_type(&self, index: usize) -> &U {
        let len = self.imports.len();
        if index < len {
            &self.imports[index].get_type()
        } else {
            &self.defines[index - len].get_type()
        }
    }

    pub fn is_import(&self, index: usize) -> bool {
        let len = self.imports.len();
        if index < len {
            true
        } else if index < self.defines.len() + len {
            false
        } else {
            unreachable!()
        }
    }

    pub fn is_define(&self, index: usize) -> bool {
        !self.is_import(index)
    }
}

pub struct Memory;
pub struct Table;

#[derive(Debug)]
pub struct Global(GlobalType);

impl From<parity_wasm::elements::GlobalEntry> for Global {
    fn from(v: parity_wasm::elements::GlobalEntry) -> Global {
        Global(GlobalType::from(*v.global_type()))
    }
}

impl Entry<GlobalType> for Global {
    fn get_type(&self) -> &GlobalType {
        &self.0
    }
}
impl Def<GlobalType> for Global {}

pub struct Module {
    types: Vec<FunctionType>,
    functions: CombinedDeclear<Function, FunctionType>,
    globals: CombinedDeclear<Global, GlobalType>,
}

impl From<parity_wasm::elements::Module> for Module {
    fn from(module: parity_wasm::elements::Module) -> Self {
        let func_types = match module.type_section() {
            None => Vec::new(),
            Some(section) => section
                .types()
                .iter()
                .map(|t| match t {
                    parity_wasm::elements::Type::Function(ty) => FunctionType::from(ty.clone()),
                })
                .collect(),
        };

        let global_imports = match module.import_section() {
            None => Vec::new(),
            Some(section) => section
                .entries()
                .iter()
                .filter_map(|t| {
                    if let parity_wasm::elements::External::Global(global_ty) = t.external() {
                        Some(*global_ty)
                    } else {
                        None
                    }
                })
                .map(|t| Import(GlobalType::from(t)))
                .collect(),
        };

        let global_defs = match module.global_section() {
            None => vec![],
            Some(section) => section
                .entries()
                .iter()
                .map(|t| Global::from(t.clone()))
                .collect(),
        };

        let func_defs = match module.function_section() {
            None => &[],
            Some(section) => section.entries(),
        };

        let func_imports = match module.import_section() {
            None => vec![],
            Some(section) => section
                .entries()
                .iter()
                .filter_map(|t| {
                    if let parity_wasm::elements::External::Function(index) = t.external() {
                        Some(*index)
                    } else {
                        None
                    }
                })
                .map(|t| Import(func_types[t as usize].clone()))
                .collect(),
        };

        let func_bodys = match module.code_section() {
            None => &[],
            Some(section) => section.bodies(),
        };

        assert!(func_bodys.len() == func_defs.len());

        let functions = func_defs
            .iter()
            .zip(func_bodys.iter())
            .map(|(def, body)| Function::new(&func_types, *def, body.clone()))
            .collect();

        Self {
            types: func_types,
            functions: CombinedDeclear {
                defines: functions,
                imports: func_imports,
            },
            globals: CombinedDeclear {
                defines: global_defs,
                imports: global_imports,
            },
        }
    }
}

impl Module {
    #[inline]
    pub fn get_func_type(&self, index: u32) -> &FunctionType {
        &self.types[index as usize]
    }

    #[inline]
    pub fn types_count(&self) -> usize {
        self.types.len()
    }

    #[inline]
    pub fn functions(&self) -> &CombinedDeclear<Function, FunctionType> {
        &self.functions
    }

    #[inline]
    pub fn function_defs(&self) -> &[Function] {
        &self.functions.defines
    }

    pub fn globals(&self) -> &CombinedDeclear<Global, GlobalType> {
        &self.globals
    }
}
