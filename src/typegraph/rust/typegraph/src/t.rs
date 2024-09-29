// Copyright Metatype OÜ, licensed under the Elastic License 2.0.
// SPDX-License-Identifier: Elastic-2.0

use serde::Serialize;
use serde_json::Value;

use crate::{
    error::Result,
    injections::{serialize_injection, InjectionSource},
    policy::AsPolicyChain,
    utils::json_stringify,
    wasm::{
        self,
        core::{
            FuncParams, MaterializerId, ParameterTransform, TypeBase, TypeEither, TypeFloat,
            TypeFunc, TypeId, TypeInteger, TypeList, TypeOptional, TypeString, TypeStruct,
            TypeUnion,
        },
    },
};

#[derive(Debug, Clone, Copy)]
pub struct TypeDef {
    id: TypeId,
}

impl TypeDef {
    pub fn id(&self) -> TypeId {
        self.id
    }

    pub fn repr(&self) -> Result<String> {
        wasm::with_core(|c, s| c.call_get_type_repr(s, self.id))
    }

    pub fn rename(mut self, name: &str) -> Result<Self> {
        self.id = wasm::with_core(|c, s| c.call_rename_type(s, self.id, name))?;
        Ok(self)
    }

    pub fn with_policy<P>(mut self, policy: &P) -> Result<Self>
    where
        P: AsPolicyChain,
    {
        self.id = wasm::with_core(|c, s| c.call_with_policy(s, self.id, &policy.as_chain()))?;
        Ok(self)
    }

    pub fn set<I>(self, value: I) -> Result<Self>
    where
        I: Into<Value>,
    {
        self.with_injection(serialize_injection(InjectionSource::Static, Some(value)))
    }

    pub fn inject(self, value: &str) -> Result<Self> {
        self.with_injection(serialize_injection(InjectionSource::Dynamic, Some(value)))
    }

    // TODO
    // pub fn from_parent(self, value: &str) -> Result<Self> {}

    pub fn from_context(self, value: &str) -> Result<Self> {
        self.with_injection(serialize_injection(InjectionSource::Context, Some(value)))
    }

    pub fn from_secret(self, value: &str) -> Result<Self> {
        self.with_injection(serialize_injection(InjectionSource::Secret, Some(value)))
    }

    pub fn from_random(self) -> Result<Self> {
        self.with_injection(serialize_injection(InjectionSource::Random, None::<String>))
    }

    fn with_injection(mut self, injection: String) -> Result<Self> {
        self.id = wasm::with_core(|c, s| c.call_with_injection(s, self.id, &injection))?;
        Ok(self)
    }

    // TODO per-effect injections
}

pub trait TypeBuilder: Sized {
    fn build(self) -> Result<TypeDef>;

    fn into_id(self) -> Result<TypeId> {
        self.build().map(|ty| ty.id)
    }

    fn rename(self, name: &str) -> Result<TypeDef> {
        self.build()?.rename(name)
    }

    fn with_policy<P>(self, policy: &P) -> Result<TypeDef>
    where
        P: AsPolicyChain,
    {
        self.build()?.with_policy(policy)
    }

    fn set<I>(self, value: I) -> Result<TypeDef>
    where
        I: Into<Value>,
    {
        self.build()?
            .with_injection(serialize_injection(InjectionSource::Static, Some(value)))
    }

    fn inject(self, value: &str) -> Result<TypeDef> {
        self.build()?
            .with_injection(serialize_injection(InjectionSource::Dynamic, Some(value)))
    }

    fn from_context(self, value: &str) -> Result<TypeDef> {
        self.build()?
            .with_injection(serialize_injection(InjectionSource::Context, Some(value)))
    }

    fn from_secret(self, value: &str) -> Result<TypeDef> {
        self.build()?
            .with_injection(serialize_injection(InjectionSource::Secret, Some(value)))
    }

    fn from_random(self) -> Result<TypeDef> {
        self.build()?
            .with_injection(serialize_injection(InjectionSource::Random, None::<String>))
    }

    fn optional(self) -> Result<OptionalBuilder> {
        optional(self)
    }

    fn optional_or<S: Serialize>(self, value: S) -> Result<OptionalBuilder> {
        optional(self).map(|o| o.default_item(value))
    }
}

pub trait BaseBuilder: Sized {
    fn base_mut(&mut self) -> &mut TypeBase;

    fn named(mut self, name: &str) -> Self {
        self.base_mut().name = name.to_string().into();
        self
    }

    fn id(mut self) -> Self {
        self.base_mut().as_id = true;
        self
    }

    fn config<S: Serialize>(mut self, key: &str, value: S) -> Self {
        self.base_mut()
            .runtime_config
            .get_or_insert_with(Vec::new)
            .push((key.to_owned(), json_stringify(&value)));
        self
    }
}

impl TypeBuilder for TypeId {
    fn build(self) -> Result<TypeDef> {
        Ok(TypeDef { id: self })
    }
}

impl TypeBuilder for &str {
    fn build(self) -> Result<TypeDef> {
        r#ref(self).build()
    }
}

impl TypeBuilder for TypeDef {
    fn build(self) -> Result<TypeDef> {
        Ok(self)
    }
}

impl<T: TypeBuilder> TypeBuilder for Result<T> {
    fn build(self) -> Result<TypeDef> {
        self.and_then(|ty| ty.build())
    }
}

#[allow(clippy::derivable_impls)]
impl Default for TypeBase {
    fn default() -> Self {
        Self {
            name: None,
            runtime_config: None,
            as_id: false,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct BooleanBuilder {
    base: TypeBase,
}

impl TypeBuilder for BooleanBuilder {
    fn build(self) -> Result<TypeDef> {
        Ok(TypeDef {
            id: wasm::with_core(|c, s| c.call_booleanb(s, &self.base))?,
        })
    }
}

impl BaseBuilder for BooleanBuilder {
    fn base_mut(&mut self) -> &mut TypeBase {
        &mut self.base
    }
}

pub fn boolean() -> BooleanBuilder {
    BooleanBuilder::default()
}

#[allow(clippy::derivable_impls)]
impl Default for TypeInteger {
    fn default() -> Self {
        Self {
            min: None,
            max: None,
            exclusive_minimum: None,
            exclusive_maximum: None,
            multiple_of: None,
            enumeration: None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct IntegerBuilder {
    data: TypeInteger,
    base: TypeBase,
}

impl IntegerBuilder {
    pub fn min(mut self, min: i32) -> Self {
        self.data.min = Some(min);
        self
    }

    pub fn max(mut self, max: i32) -> Self {
        self.data.max = Some(max);
        self
    }

    pub fn x_min(mut self, min: i32) -> Self {
        self.data.exclusive_minimum = Some(min);
        self
    }

    pub fn x_max(mut self, max: i32) -> Self {
        self.data.exclusive_maximum = Some(max);
        self
    }

    pub fn multiple(mut self, n: i32) -> Self {
        self.data.multiple_of = Some(n);
        self
    }

    pub fn enumerate(mut self, values: impl IntoIterator<Item = i32>) -> Self {
        self.data.enumeration = Some(values.into_iter().collect());
        self
    }
}

impl TypeBuilder for IntegerBuilder {
    fn build(self) -> Result<TypeDef> {
        Ok(TypeDef {
            id: wasm::with_core(|c, s| c.call_integerb(s, &self.data, &self.base))?,
        })
    }
}

impl BaseBuilder for IntegerBuilder {
    fn base_mut(&mut self) -> &mut TypeBase {
        &mut self.base
    }
}

pub fn integer() -> IntegerBuilder {
    IntegerBuilder::default()
}

#[allow(clippy::derivable_impls)]
impl Default for TypeFloat {
    fn default() -> Self {
        Self {
            min: None,
            max: None,
            exclusive_minimum: None,
            exclusive_maximum: None,
            multiple_of: None,
            enumeration: None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct FloatBuilder {
    base: TypeBase,
    data: TypeFloat,
}

impl FloatBuilder {
    pub fn min(mut self, min: f64) -> Self {
        self.data.min = Some(min);
        self
    }

    pub fn max(mut self, max: f64) -> Self {
        self.data.max = Some(max);
        self
    }

    pub fn x_min(mut self, min: f64) -> Self {
        self.data.exclusive_minimum = Some(min);
        self
    }

    pub fn x_max(mut self, max: f64) -> Self {
        self.data.exclusive_maximum = Some(max);
        self
    }

    pub fn multiple(mut self, n: f64) -> Self {
        self.data.multiple_of = Some(n);
        self
    }

    pub fn enumerate(mut self, values: impl IntoIterator<Item = f64>) -> Self {
        self.data.enumeration = Some(values.into_iter().collect());
        self
    }
}

impl TypeBuilder for FloatBuilder {
    fn build(self) -> Result<TypeDef> {
        Ok(TypeDef {
            id: wasm::with_core(|c, s| c.call_floatb(s, &self.data, &self.base))?,
        })
    }
}

impl BaseBuilder for FloatBuilder {
    fn base_mut(&mut self) -> &mut TypeBase {
        &mut self.base
    }
}

pub fn float() -> FloatBuilder {
    FloatBuilder::default()
}

#[allow(clippy::derivable_impls)]
impl Default for TypeString {
    fn default() -> Self {
        Self {
            min: None,
            max: None,
            format: None,
            pattern: None,
            enumeration: None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct StringBuilder {
    base: TypeBase,
    data: TypeString,
}

impl StringBuilder {
    pub fn min(mut self, min: u32) -> Self {
        self.data.min = Some(min);
        self
    }

    pub fn max(mut self, max: u32) -> Self {
        self.data.max = Some(max);
        self
    }

    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.data.format = Some(format.into());
        self
    }

    pub fn enumerate(mut self, values: impl IntoIterator<Item = impl ToString>) -> Self {
        self.data.enumeration = Some(values.into_iter().map(|v| v.to_string()).collect());
        self
    }
}

impl TypeBuilder for StringBuilder {
    fn build(self) -> Result<TypeDef> {
        Ok(TypeDef {
            id: wasm::with_core(|c, s| c.call_stringb(s, &self.data, &self.base))?,
        })
    }
}

impl BaseBuilder for StringBuilder {
    fn base_mut(&mut self) -> &mut TypeBase {
        &mut self.base
    }
}

pub fn string() -> StringBuilder {
    StringBuilder::default()
}

pub fn uuid() -> StringBuilder {
    string().id().format("uuid")
}

pub fn email() -> StringBuilder {
    string().format("email")
}

pub fn uri() -> StringBuilder {
    string().format("uri")
}

pub fn ean() -> StringBuilder {
    string().format("ean")
}

pub fn date() -> StringBuilder {
    string().format("date")
}

pub fn datetime() -> StringBuilder {
    string().format("datetime")
}

pub fn json() -> StringBuilder {
    string().format("json")
}

impl Default for TypeOptional {
    fn default() -> Self {
        Self {
            of: u32::MAX,
            default_item: None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct OptionalBuilder {
    base: TypeBase,
    data: TypeOptional,
}

impl OptionalBuilder {
    pub fn default_item<S: Serialize>(mut self, value: S) -> Self {
        self.data.default_item = json_stringify(&value).into();
        self
    }
}

impl TypeBuilder for OptionalBuilder {
    fn build(self) -> Result<TypeDef> {
        Ok(TypeDef {
            id: wasm::with_core(|c, s| c.call_optionalb(s, &self.data, &self.base))?,
        })
    }
}

impl BaseBuilder for OptionalBuilder {
    fn base_mut(&mut self) -> &mut TypeBase {
        &mut self.base
    }
}

pub fn optional(ty: impl TypeBuilder) -> Result<OptionalBuilder> {
    Ok(OptionalBuilder {
        base: TypeBase::default(),
        data: TypeOptional {
            of: ty.into_id()?,
            default_item: None,
        },
    })
}

impl Default for TypeList {
    fn default() -> Self {
        Self {
            of: u32::MAX,
            min: None,
            max: None,
            unique_items: None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ListBuilder {
    base: TypeBase,
    data: TypeList,
}

impl ListBuilder {
    pub fn min(mut self, min: u32) -> Self {
        self.data.min = Some(min);
        self
    }

    pub fn max(mut self, max: u32) -> Self {
        self.data.max = Some(max);
        self
    }

    pub fn unique(mut self, value: bool) -> Self {
        self.data.unique_items = Some(value);
        self
    }
}

impl TypeBuilder for ListBuilder {
    fn build(self) -> Result<TypeDef> {
        Ok(TypeDef {
            id: wasm::with_core(|c, s| c.call_listb(s, self.data, &self.base))?,
        })
    }
}

impl BaseBuilder for ListBuilder {
    fn base_mut(&mut self) -> &mut TypeBase {
        &mut self.base
    }
}

pub fn list(ty: impl TypeBuilder) -> Result<ListBuilder> {
    Ok(ListBuilder {
        base: TypeBase::default(),
        data: TypeList {
            of: ty.into_id()?,
            ..Default::default()
        },
    })
}

#[allow(clippy::derivable_impls)]
impl Default for TypeUnion {
    fn default() -> Self {
        Self {
            variants: Vec::default(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct UnionBuilder {
    base: TypeBase,
    data: TypeUnion,
}

impl UnionBuilder {
    pub fn add(mut self, ty: impl TypeBuilder) -> Result<Self> {
        self.data.variants.push(ty.into_id()?);
        Ok(self)
    }
}

impl TypeBuilder for UnionBuilder {
    fn build(self) -> Result<TypeDef> {
        Ok(TypeDef {
            id: wasm::with_core(|c, s| c.call_unionb(s, &self.data, &self.base))?,
        })
    }
}

impl BaseBuilder for UnionBuilder {
    fn base_mut(&mut self) -> &mut TypeBase {
        &mut self.base
    }
}

pub fn union(variants: impl IntoIterator<Item = impl TypeBuilder>) -> Result<UnionBuilder> {
    Ok(UnionBuilder {
        data: TypeUnion {
            variants: variants
                .into_iter()
                .map(|ty| ty.into_id())
                .collect::<Result<_>>()?,
        },
        ..Default::default()
    })
}

#[allow(clippy::derivable_impls)]
impl Default for TypeEither {
    fn default() -> Self {
        Self {
            variants: Vec::default(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct EitherBuilder {
    base: TypeBase,
    data: TypeEither,
}

impl TypeBuilder for EitherBuilder {
    fn build(self) -> Result<TypeDef> {
        Ok(TypeDef {
            id: wasm::with_core(|c, s| c.call_eitherb(s, &self.data, &self.base))?,
        })
    }
}

impl BaseBuilder for EitherBuilder {
    fn base_mut(&mut self) -> &mut TypeBase {
        &mut self.base
    }
}

pub fn either(variants: impl IntoIterator<Item = impl TypeBuilder>) -> Result<EitherBuilder> {
    Ok(EitherBuilder {
        data: TypeEither {
            variants: variants
                .into_iter()
                .map(|ty| ty.into_id())
                .collect::<Result<_>>()?,
        },
        ..Default::default()
    })
}

#[allow(clippy::derivable_impls)]
impl Default for TypeStruct {
    fn default() -> Self {
        Self {
            props: Vec::new(),
            additional_props: false,
            min: None,
            max: None,
            enumeration: None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct StructBuilder {
    base: TypeBase,
    data: TypeStruct,
}

impl StructBuilder {
    pub fn prop(mut self, name: impl ToString, ty: impl TypeBuilder) -> Result<Self> {
        self.data.props.push((name.to_string(), ty.into_id()?));
        Ok(self)
    }

    pub fn min(mut self, min: u32) -> Self {
        self.data.min = Some(min);
        self
    }

    pub fn max(mut self, max: u32) -> Self {
        self.data.max = Some(max);
        self
    }
}

impl TypeBuilder for StructBuilder {
    fn build(self) -> Result<TypeDef> {
        Ok(TypeDef {
            id: wasm::with_core(|c, s| c.call_structb(s, &self.data, &self.base))?,
        })
    }
}

impl BaseBuilder for StructBuilder {
    fn base_mut(&mut self) -> &mut TypeBase {
        &mut self.base
    }
}

pub fn r#struct() -> StructBuilder {
    StructBuilder::default()
}

impl Default for TypeFunc {
    fn default() -> Self {
        Self {
            inp: u32::MAX,
            out: u32::MAX,
            parameter_transform: None,
            mat: u32::MAX,
            rate_calls: false,
            rate_weight: None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct FuncBuilder {
    data: TypeFunc,
}

impl FuncBuilder {
    pub fn rate_call(mut self, value: bool) -> Self {
        self.data.rate_calls = value;
        self
    }

    pub fn rate_weight(mut self, value: u32) -> Self {
        self.data.rate_weight = Some(value);
        self
    }

    pub fn transform(mut self, transform: ParameterTransform) -> Self {
        self.data.parameter_transform = Some(transform);
        self
    }
}

impl TypeBuilder for FuncBuilder {
    fn build(self) -> Result<TypeDef> {
        Ok(TypeDef {
            id: wasm::with_core(|c, s| c.call_funcb(s, &self.data))?,
        })
    }
}

pub fn func<I, O>(inp: I, out: O, mat: MaterializerId) -> Result<FuncBuilder>
where
    I: TypeBuilder,
    O: TypeBuilder,
{
    Ok(FuncBuilder {
        data: TypeFunc {
            inp: inp.into_id()?,
            out: out.into_id()?,
            mat,
            ..Default::default()
        },
    })
}

pub fn funcb<I, O>(inp: I, out: O, mat: MaterializerId) -> Result<TypeDef>
where
    I: TypeBuilder,
    O: TypeBuilder,
{
    func(inp, out, mat).build()
}

impl TypeBuilder for FuncParams {
    fn build(self) -> Result<TypeDef> {
        func(self.inp, self.out, self.mat)?.build()
    }
}

#[derive(Debug, Clone)]
pub struct RefBuilder {
    name: String,
    attributes: Vec<(String, String)>,
}

impl RefBuilder {
    pub fn set(mut self, attr: &str, value: &str) -> Self {
        self.attributes.push((attr.to_string(), value.to_string()));
        self
    }
}

impl TypeBuilder for RefBuilder {
    fn build(self) -> Result<TypeDef> {
        Ok(TypeDef {
            id: wasm::with_core(|c, s| c.call_refb(s, &self.name, &self.attributes))?,
        })
    }
}

pub fn r#ref(name: &str) -> RefBuilder {
    RefBuilder {
        name: name.to_string(),
        attributes: vec![],
    }
}
