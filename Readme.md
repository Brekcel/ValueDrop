This crate is a helper utility for structs that need to drop using self instead of&mut self as provided by Drop.
This crate contains 2 things:
1. The trait ValueDrop. Types that need to drop using self should implement this trait.
2. The struct AutoValueDrop. This struct will automatically call ValueDrop.drop on it'scontents when this struct is dropped by normal Rusty means. It implements Derefand DerefMut so it should be possible to use this as if it's the normal struct.It also implements, Debug, Clone, Default,Eq, PartialEq, Ord, PartialOrd, and Hash when possible.
This crate is no_std by default.
# Example
```rust,no_runuse c_crate_sys::{init_c_data, free_c_data};use selfdrop::{AutoValueDrop, ValueDrop};
struct CWrapper {    data: CData,    argument: usize}
impl ValueDrop for CWrapper {    fn drop(self) {        //free_c_data's definition is fn free_c_data(data: CData, argument: usize);        //As such, you cannot free this data from the normal Drop        //without wrapping it in an Option or similar        free_c_data(self.data, self.argument);    }}
impl CWrapper {    pub fn new(argument: usize) -> AutoValueDrop<CWrapper> {        let data: CData = init_c_data(argument);        let wrapper: CWrapper = CWrapper {data, argument};        AutoValueDrop::new(wrapper)    }}```