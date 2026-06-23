use zed_extension_api as zed;

struct BitburnerExtension;

impl zed::Extension for BitburnerExtension {
    fn new() -> Self {
        Self
    }
}

zed::register_extension!(BitburnerExtension);
