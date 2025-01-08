pub trait Device {
    const NAME: &'static str = "UNNAMED";

    /// Only reads lower nibble of byte.
    fn read(&mut self) -> u8;

    /// Only sends lower nibble of byte.
    fn write(&mut self, data: u8);

    /// TODO Remove, only used for debugging
    fn debug_poll(&mut self) {}

    fn name(&self) -> &'static str {
        Self::NAME
    }
}
