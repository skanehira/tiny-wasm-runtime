(module
  (func $add (import "env" "add") (param i32) (result i32))
  (func (export "call_add") (param i32) (result i32)
    (local.get 0)
    (call $add)
  )
)
