// A mob's name taken away (or hidden again) goes out as none / false.
fn name_defaults(defaults: &mut Vec<(u8, i32, Value)>) {
    defaults.push((2, serializer::OPTIONAL_COMPONENT, Value::OptText(None)));
    defaults.push((3, serializer::BOOLEAN, Value::Bool(false)));
}
