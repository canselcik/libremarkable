/// Contains the glue code connecting the Lua context to the methods used for IO
pub mod luaext;

/// Contains the types used by the `ApplicationContext` such as `UIElementWrapper`, etc.
/// None of these are mandatory to be used. You can choose to entirely ignore the
/// `ApplicationContext` and `ui_extensions` and choose to interact with the `framebuffer`
/// and `input` devices directly.
pub mod element;
