use crate::{
    t::{self, TypeBuilder},
    wasm::{
        self,
        core::TypeId,
        runtimes::{
            BaseMaterializer, Effect, HttpMethod, HttpRuntimeData, MaterializerHttpRequest,
            RuntimeId,
        },
    },
    Result,
};

impl Default for HttpMethod {
    fn default() -> Self {
        Self::Get
    }
}

#[derive(Debug, Default)]
pub struct HttpRequestBuilder {
    runtime: RuntimeId,
    inp: TypeId,
    out: TypeId,
    method: HttpMethod,
    path: String,
    content_type: Option<String>,
    header_prefix: Option<String>,
    query_fields: Option<Vec<String>>,
    effect: Effect,
}

impl TypeBuilder for HttpRequestBuilder {
    fn build(self) -> Result<TypeId> {
        let base = BaseMaterializer {
            runtime: self.runtime,
            effect: self.effect,
        };

        let mat_options = MaterializerHttpRequest {
            method: self.method,
            path: self.path,
            content_type: self.content_type,
            header_prefix: self.header_prefix,
            query_fields: self.query_fields,
            rename_fields: None,
            body_fields: None,
            auth_token_field: None,
        };

        let mat_id = wasm::with_runtimes(|r, s| r.call_http_request(s, base, &mat_options))?;

        t::func(self.inp.build()?, self.out.build()?, mat_id)?.build()
    }
}

impl HttpRequestBuilder {
    pub fn content_type(mut self, content: impl ToString) -> Self {
        self.content_type = Some(content.to_string());
        self
    }

    pub fn header_prefix(mut self, prefix: impl ToString) -> Self {
        self.header_prefix = Some(prefix.to_string());
        self
    }

    pub fn query_fields(mut self, fields: impl IntoIterator<Item = impl ToString>) -> Self {
        self.header_prefix = Some(fields.into_iter().map(|f| f.to_string()).collect());
        self
    }

    pub fn effect(mut self, effect: Effect) -> Self {
        self.effect = effect;
        self
    }
}

#[derive(Debug)]
pub struct HttpRuntime {
    id: u32,
}

impl HttpRuntime {
    pub fn new<S>(endpoint: S, cert_secret: Option<S>, basic_auth_secret: Option<S>) -> Result<Self>
    where
        S: ToString,
    {
        let id = wasm::with_runtimes(|r, s| {
            r.call_register_http_runtime(
                s,
                &HttpRuntimeData {
                    endpoint: endpoint.to_string(),
                    cert_secret: cert_secret.as_ref().map(|v| v.to_string()),
                    basic_auth_secret: basic_auth_secret.as_ref().map(|v| v.to_string()),
                },
            )
        })?;

        Ok(Self { id })
    }

    pub fn get<T: TypeBuilder>(&self, inp: T, out: T) -> Result<HttpRequestBuilder> {
        self.request(HttpMethod::Get, inp, out)
    }

    pub fn post<T: TypeBuilder>(&self, inp: T, out: T) -> Result<HttpRequestBuilder> {
        self.request(HttpMethod::Post, inp, out)
            .map(|r| r.effect(Effect::Create(false)))
    }

    pub fn put<T: TypeBuilder>(&self, inp: T, out: T) -> Result<HttpRequestBuilder> {
        self.request(HttpMethod::Put, inp, out)
            .map(|r| r.effect(Effect::Update(false)))
    }

    pub fn patch<T: TypeBuilder>(&self, inp: T, out: T) -> Result<HttpRequestBuilder> {
        self.request(HttpMethod::Patch, inp, out)
            .map(|r| r.effect(Effect::Update(false)))
    }

    pub fn delete<T: TypeBuilder>(&self, inp: T, out: T) -> Result<HttpRequestBuilder> {
        self.request(HttpMethod::Delete, inp, out)
            .map(|r| r.effect(Effect::Delete(false)))
    }

    fn request<T: TypeBuilder>(
        &self,
        method: HttpMethod,
        inp: T,
        out: T,
    ) -> Result<HttpRequestBuilder> {
        Ok(HttpRequestBuilder {
            runtime: self.id,
            method,
            inp: inp.build()?,
            out: out.build()?,
            ..Default::default()
        })
    }
}
