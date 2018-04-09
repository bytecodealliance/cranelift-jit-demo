use std::collections::HashMap;

use cretonne::entity::EntityRef;
use cretonne::ir::{AbiParam, InstBuilder, Value, Ebb, Signature, CallConv};
use cretonne::ir::types;
use cretonne::ir::condcodes::IntCC;
use cretonne;
use cton_frontend::{FunctionBuilderContext, FunctionBuilder, Variable};
use cton_module;
use cton_simplejit;

pub enum Expr {
    Literal(String),
    Identifier(String),
    Assign(String, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    If(Box<Expr>, Vec<Expr>, Vec<Expr>),
    Call(String, Vec<Expr>),
}

mod parser {
    include!(concat!(env!("OUT_DIR"), "/grammar.rs"));
}

pub struct JIT {
    builder_context: FunctionBuilderContext<Variable>,
    ctx: cretonne::Context,
    module: cton_module::Module<cton_simplejit::SimpleJITBackend>,
}

impl JIT {
    pub fn new() -> Self {
        let backend = cton_simplejit::SimpleJITBackend::new();
        Self {
            builder_context: FunctionBuilderContext::<Variable>::new(),
            ctx: cretonne::Context::new(),
            module: cton_module::Module::new(backend),
        }
    }

    pub fn compile(&mut self, input: &str) -> Result<*const u8, String> {
        let (name, params, the_return, stmts) =
            parser::function(&input).map_err(|e| e.to_string())?;

        self.translate(params, the_return, stmts).map_err(
            |e| e.to_string(),
        )?;
        let id = self.module
            .declare_function(
                &name,
                cton_module::Linkage::Export,
                &self.ctx.func.signature,
            )
            .map_err(|e| e.to_string())?;
        self.module.define_function(id, &mut self.ctx).map_err(
            |e| {
                e.to_string()
            },
        )?;
        self.ctx.clear();
        let code = self.module.finalize_function(id);

        Ok(code)
    }

    fn translate(
        &mut self,
        params: Vec<String>,
        the_return: String,
        stmts: Vec<Expr>,
    ) -> Result<(), String> {
        for _p in &params {
            self.ctx.func.signature.params.push(
                AbiParam::new(types::I32),
            );
        }
        self.ctx.func.signature.returns.push(
            AbiParam::new(types::I32),
        );

        let mut builder =
            FunctionBuilder::<Variable>::new(&mut self.ctx.func, &mut self.builder_context);

        // TODO: Streamline the API here.
        let entry_ebb = builder.create_ebb();
        builder.append_ebb_params_for_function_params(entry_ebb);
        builder.switch_to_block(entry_ebb);
        builder.seal_block(entry_ebb);

        let variables = declare_variables(&mut builder, &params, &the_return, &stmts, entry_ebb);

        let mut trans = FunctionTranslator {
            builder,
            variables,
            module: &mut self.module,
        };
        for expr in stmts {
            trans.translate_expr(expr);
        }
        let return_variable = trans.variables.get(&the_return).unwrap();
        let return_value = trans.builder.use_var(*return_variable);
        trans.builder.ins().return_(&[return_value]);

        trans.builder.finalize();
        Ok(())
    }
}

struct FunctionTranslator<'a> {
    builder: FunctionBuilder<'a, Variable>,
    variables: HashMap<String, Variable>,
    module: &'a mut cton_module::Module<cton_simplejit::SimpleJITBackend>,
}

impl<'a> FunctionTranslator<'a> {
    // When you write out instructions in Cretonne, you get back `Value`s. You
    // can then use these references in other instructions.
    fn translate_expr(&mut self, expr: Expr) -> Value {
        match expr {
            Expr::Literal(literal) => {
                let imm: i32 = literal.parse().unwrap();
                self.builder.ins().iconst(types::I32, i64::from(imm))
            }

            Expr::Add(lhs, rhs) => {
                let lhs = self.translate_expr(*lhs);
                let rhs = self.translate_expr(*rhs);
                self.builder.ins().iadd(lhs, rhs)
            }

            Expr::Sub(lhs, rhs) => {
                let lhs = self.translate_expr(*lhs);
                let rhs = self.translate_expr(*rhs);
                self.builder.ins().isub(lhs, rhs)
            }

            Expr::Mul(lhs, rhs) => {
                let lhs = self.translate_expr(*lhs);
                let rhs = self.translate_expr(*rhs);
                self.builder.ins().imul(lhs, rhs)
            }

            Expr::Div(lhs, rhs) => {
                let lhs = self.translate_expr(*lhs);
                let rhs = self.translate_expr(*rhs);
                self.builder.ins().udiv(lhs, rhs)
            }

            Expr::Eq(lhs, rhs) => {
                let lhs = self.translate_expr(*lhs);
                let rhs = self.translate_expr(*rhs);
                let c = self.builder.ins().icmp(IntCC::Equal, lhs, rhs);
                self.builder.ins().bint(types::I32, c)
            }

            Expr::Ne(lhs, rhs) => {
                let lhs = self.translate_expr(*lhs);
                let rhs = self.translate_expr(*rhs);
                let c = self.builder.ins().icmp(IntCC::NotEqual, lhs, rhs);
                self.builder.ins().bint(types::I32, c)
            }

            Expr::Lt(lhs, rhs) => {
                let lhs = self.translate_expr(*lhs);
                let rhs = self.translate_expr(*rhs);
                let c = self.builder.ins().icmp(IntCC::SignedLessThan, lhs, rhs);
                self.builder.ins().bint(types::I32, c)
            }

            Expr::Le(lhs, rhs) => {
                let lhs = self.translate_expr(*lhs);
                let rhs = self.translate_expr(*rhs);
                let c = self.builder.ins().icmp(
                    IntCC::SignedLessThanOrEqual,
                    lhs,
                    rhs,
                );
                self.builder.ins().bint(types::I32, c)
            }

            Expr::Gt(lhs, rhs) => {
                let lhs = self.translate_expr(*lhs);
                let rhs = self.translate_expr(*rhs);
                let c = self.builder.ins().icmp(IntCC::SignedGreaterThan, lhs, rhs);
                self.builder.ins().bint(types::I32, c)
            }

            Expr::Ge(lhs, rhs) => {
                let lhs = self.translate_expr(*lhs);
                let rhs = self.translate_expr(*rhs);
                let c = self.builder.ins().icmp(
                    IntCC::SignedGreaterThanOrEqual,
                    lhs,
                    rhs,
                );
                self.builder.ins().bint(types::I32, c)
            }

            Expr::Call(name, args) => self.translate_call(name, args),

            Expr::Identifier(name) => {
                let variable = self.variables.get(&name).unwrap();
                self.builder.use_var(*variable)
            }

            Expr::Assign(name, expr) => {
                let new_value = self.translate_expr(*expr);
                let variable = self.variables.get(&name).unwrap();
                self.builder.def_var(*variable, new_value);
                new_value
            }

            Expr::If(condition, then_body, else_body) => {
                let condition_value = self.translate_expr(*condition);

                let else_block = self.builder.create_ebb();
                let merge_block = self.builder.create_ebb();
                self.builder.append_ebb_param(merge_block, types::I32);

                self.builder.ins().brz(condition_value, else_block, &[]);

                let mut then_return = self.builder.ins().iconst(types::I32, 0);
                for expr in then_body {
                    then_return = self.translate_expr(expr);
                }
                self.builder.ins().jump(merge_block, &[then_return]);

                self.builder.switch_to_block(else_block);
                self.builder.seal_block(else_block);
                let mut else_return = self.builder.ins().iconst(types::I32, 0);
                for expr in else_body {
                    else_return = self.translate_expr(expr);
                }
                self.builder.ins().jump(merge_block, &[else_return]);

                self.builder.switch_to_block(merge_block);
                self.builder.seal_block(merge_block);
                let phi = self.builder.ebb_params(merge_block)[0];

                phi
            }
        }
    }

    fn translate_call(&mut self, name: String, args: Vec<Expr>) -> Value {
        let mut sig = Signature::new(CallConv::SystemV);

        // Add a parameter for each argument.
        for _arg in &args {
            sig.params.push(AbiParam::new(types::I32));
        }

        // For simplicity for now, just make all calls return a single I32.
        sig.returns.push(AbiParam::new(types::I32));

        // TODO: Streamline the API here?
        let callee = self.module
            .declare_function(&name, cton_module::Linkage::Export, &sig)
            .expect("problem declaring function");
        let local_callee = self.module.declare_func_in_func(
            callee,
            &mut self.builder.func,
        );

        let mut arg_values = Vec::new();
        for arg in args {
            arg_values.push(self.translate_expr(arg))
        }
        let call = self.builder.ins().call(local_callee, &arg_values);
        self.builder.inst_results(call)[0]
    }
}

fn declare_variables(
    builder: &mut FunctionBuilder<Variable>,
    params: &[String],
    the_return: &str,
    stmts: &[Expr],
    entry_ebb: Ebb,
) -> HashMap<String, Variable> {
    let mut variables = HashMap::new();
    let mut index = 0;

    for (i, name) in params.iter().enumerate() {
        // TODO: cton_frontend should really have an API to make it easy to set
        // up param variables.
        let val = builder.ebb_params(entry_ebb)[i];
        let var = declare_variable(builder, &mut variables, &mut index, name);
        builder.def_var(var, val);
    }
    let zero = builder.ins().iconst(types::I32, 0);
    let return_variable = declare_variable(builder, &mut variables, &mut index, the_return);
    builder.def_var(return_variable, zero);
    for expr in stmts {
        if let Expr::Assign(ref name, _) = *expr {
            declare_variable(builder, &mut variables, &mut index, name);
        }
    }

    variables
}

fn declare_variable(
    builder: &mut FunctionBuilder<Variable>,
    variables: &mut HashMap<String, Variable>,
    index: &mut usize,
    name: &str,
) -> Variable {
    let var = Variable::new(*index);
    if !variables.contains_key(name) {
        variables.insert(name.into(), var);
        builder.declare_var(var, types::I32);
        *index += 1;
    }
    var
}
