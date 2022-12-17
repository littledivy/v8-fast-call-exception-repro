use v8::MapFnTo;

fn setup() {
  v8::V8::set_flags_from_string("--turbo_fast_api_calls");
  v8::V8::initialize_platform(v8::new_default_platform(0, false).make_shared());
  v8::V8::initialize();
}

fn main() {
  setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let mut scope = v8::HandleScope::new(isolate);

  let global = setup_bindings(&mut scope);
  let context = v8::Context::new_from_template(&mut scope, global);

  let filename = std::env::args()
    .nth(1)
    .expect("Invalid invocation. Usage: v8-fallback-exception <filename>");
  let source = std::fs::read_to_string(filename).expect("Failed to read file");
  eval(&source, &mut scope, context);
}


fn throw_exception(
  scope: &mut v8::HandleScope,
  _: v8::FunctionCallbackArguments,
  _: v8::ReturnValue,
) {
  let e = v8::undefined(scope).into();
  scope.throw_exception(e);
}

pub struct FastFallback;

impl v8::fast_api::FastFunction for FastFallback {
  fn function(&self) -> *const std::ffi::c_void {
      fast_fallback as *const _
  }
  fn args(&self) -> &'static [v8::fast_api::Type] {
    &[
      v8::fast_api::Type::V8Value,
      v8::fast_api::Type::CallbackOptions,
    ]
  }
  fn return_type(&self) -> v8::fast_api::CType {
    v8::fast_api::CType::Void
  }
}

fn fast_fallback(
  _: v8::Local<v8::Object>,
  options: *mut v8::fast_api::FastApiCallbackOptions,
) {
  let options = unsafe { &mut *options };
  options.fallback = true;
}

fn setup_bindings<'a, 's>(
  scope: &'a mut v8::HandleScope<'s, ()>,
) -> v8::Local<'s, v8::ObjectTemplate> {
  let global = v8::ObjectTemplate::new(scope);
  global.set(
    v8::String::new(scope, "bug").unwrap().into(),
    v8::FunctionTemplate::builder_raw(throw_exception.map_fn_to())
      .build_fast(scope, &FastFallback, None)
      .into(),
  );
  global
}

fn eval<'s>(
  source: &str,
  scope: &mut v8::HandleScope<'s, ()>,
  context: v8::Local<'s, v8::Context>,
) {
  let scope = &mut v8::ContextScope::new(scope, context);
  let source = v8::String::new(scope, source).unwrap();

  let try_catch = &mut v8::TryCatch::new(scope);

  let script = v8::Script::compile(try_catch, source, None)
    .expect("failed to compile script");

  if script.run(try_catch).is_none() {
    let exception = try_catch.exception().unwrap();
    let exception_string = exception
      .to_string(try_catch)
      .unwrap()
      .to_rust_string_lossy(try_catch);

    println!("Uncaught error in user script: {}", exception_string);
    std::process::exit(1);
  }
}

