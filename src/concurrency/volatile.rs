use core::ptr;
#[derive(Debug)]
#[repr(transparent)]
pub struct Volatile<T: Copy>(T);

impl<T: Copy> Volatile<T> {
    #[cfg(feature="const_fn")]
    pub const fn new(value: T) -> Volatile<T> {
        Volatile(value)
    }
    #[cfg(not(feature="const_fn"))]
    pub fn new(value: T) -> Volatile<T> {
        Volatile(value)
    }
    pub fn read(&self) -> T {
        unsafe { ptr::read_volatile(&self.0) }
    }
    pub fn write(&mut self, value: T) {
        unsafe { ptr::write_volatile(&mut self.0, value) };
    }
    pub fn update<F>(&mut self, f: F)
        where F: FnOnce(&mut T)
    {
        let mut value = self.read();
        f(&mut value);
        self.write(value);
    }
}

impl<T: Copy> Clone for Volatile<T> {
    fn clone(&self) -> Self {
        Volatile(self.read())
    }
}
///
/// The size of this struct is the same as the contained type.
#[derive(Debug, Clone)]
pub struct ReadOnly<T: Copy>(Volatile<T>);

impl<T: Copy> ReadOnly<T> {
    #[cfg(feature = "const_fn")]
    pub const fn new(value: T) -> ReadOnly<T> {
        ReadOnly(Volatile::new(value))
    }
    #[cfg(not(feature = "const_fn"))]
    pub fn new(value: T) -> ReadOnly<T> {
        ReadOnly(Volatile::new(value))
    }
    pub fn read(&self) -> T {
        self.0.read()
    }
}

#[derive(Debug, Clone)]
pub struct WriteOnly<T: Copy>(Volatile<T>);

impl<T: Copy> WriteOnly<T> {
    #[cfg(feature = "const_fn")]
    pub const fn new(value: T) -> WriteOnly<T> {
        WriteOnly(Volatile::new(value))
    }
    #[cfg(not(feature = "const_fn"))]
    pub fn new(value: T) -> WriteOnly<T> {
        WriteOnly(Volatile::new(value))
    }
    pub fn write(&mut self, value: T) {
        self.0.write(value)
    }
}