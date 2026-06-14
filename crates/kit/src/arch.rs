//! Architecture detection and configuration utilities
//!
//! This module provides cross-architecture support for libvirt domain creation
//! and QEMU emulator selection, avoiding hardcoded architecture assumptions.

use crate::xml_utils::XmlWriter;
use color_eyre::Result;

/// Architecture configuration for libvirt domains and QEMU
#[derive(Debug, Clone)]
pub struct ArchConfig {
    /// Architecture string for libvirt (e.g., "x86_64", "aarch64")
    pub arch: &'static str,
    /// Machine type for libvirt (e.g., "q35", "virt")
    pub machine: &'static str,
    /// OS type for libvirt (usually "hvm")
    pub os_type: &'static str,
}

impl ArchConfig {
    /// Detect host architecture and return appropriate configuration
    pub fn detect() -> Result<Self> {
        let arch = std::env::consts::ARCH;
        match arch {
            "x86_64" => Ok(Self {
                arch: "x86_64",
                machine: "q35",
                os_type: "hvm",
            }),
            "aarch64" => Ok(Self {
                arch: "aarch64",
                machine: "virt",
                os_type: "hvm",
            }),
            // Add more architectures as needed
            // "riscv64" => Ok(Self {
            //     arch: "riscv64",
            //     machine: "virt",
            //     os_type: "hvm",
            // }),
            unsupported => Err(color_eyre::eyre::eyre!(
                "Unsupported architecture: {}. Supported architectures: x86_64, aarch64",
                unsupported
            )),
        }
    }

    /// Generate architecture-specific timer configuration
    pub fn write_timers(&self, writer: &mut XmlWriter) -> Result<()> {
        // RTC timer is common to all architectures
        writer.write_empty_element("timer", &[("name", "rtc"), ("tickpolicy", "catchup")])?;

        // Add x86_64-specific timers
        if self.arch == "x86_64" {
            writer.write_empty_element("timer", &[("name", "pit"), ("tickpolicy", "delay")])?;
            writer.write_empty_element("timer", &[("name", "hpet"), ("present", "no")])?;
        }

        Ok(())
    }

    /// Check if this architecture supports VMport (x86_64 specific feature)
    #[allow(dead_code)]
    pub fn supports_vmport(&self) -> bool {
        self.arch == "x86_64"
    }

    /// Get recommended CPU mode for this architecture
    pub fn cpu_mode(&self) -> &'static str {
        match self.arch {
            "x86_64" => "host-passthrough",
            "aarch64" => "host-passthrough",
            _ => "host-model",
        }
    }
}

/// Detect host architecture string (shorthand for ArchConfig::detect().arch)
#[allow(dead_code)]
pub fn host_arch() -> Result<&'static str> {
    Ok(ArchConfig::detect()?.arch)
}

/// Check if running on x86_64 architecture
#[allow(dead_code)]
pub fn is_x86_64() -> bool {
    std::env::consts::ARCH == "x86_64"
}

/// Check if running on ARM64/AArch64 architecture
#[allow(dead_code)]
pub fn is_aarch64() -> bool {
    std::env::consts::ARCH == "aarch64"
}

/// Normalize an architecture name to the QEMU spelling used by
/// `qemu-system-<arch>` and `std::env::consts::ARCH`.
///
/// Accepts both OCI/Go names (`amd64`, `arm64`) and QEMU/Rust names
/// (`x86_64`, `aarch64`). Unknown values are returned unchanged.
pub fn qemu_arch(input: &str) -> String {
    match input {
        "amd64" | "x86_64" => "x86_64",
        "arm64" | "aarch64" => "aarch64",
        "ppc64le" | "powerpc64le" | "powerpc64" => "powerpc64",
        "s390x" => "s390x",
        other => other,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arch_detection() {
        let arch_config = ArchConfig::detect().unwrap();

        // Should detect the current architecture
        assert_eq!(arch_config.arch, std::env::consts::ARCH);

        // Should have valid configuration
        assert!(!arch_config.machine.is_empty());
        assert_eq!(arch_config.os_type, "hvm");
    }

    #[test]
    fn test_arch_specific_features() {
        let arch_config = ArchConfig::detect().unwrap();

        // Test that we can generate timers XML without errors
        let mut writer = XmlWriter::new();
        arch_config.write_timers(&mut writer).unwrap();
        let timers_xml = writer.into_string().unwrap();
        assert!(timers_xml.contains("timer"));
        assert!(timers_xml.contains("rtc"));

        // CPU mode should be valid
        assert!(!arch_config.cpu_mode().is_empty());
    }

    #[test]
    fn test_vmport_support() {
        let arch_config = ArchConfig::detect().unwrap();

        // VMport support should match architecture
        if arch_config.arch == "x86_64" {
            assert!(arch_config.supports_vmport());
        } else {
            assert!(!arch_config.supports_vmport());
        }
    }

    #[test]
    fn test_helper_functions() {
        let detected_arch = host_arch().unwrap();
        assert_eq!(detected_arch, std::env::consts::ARCH);

        // At least one should be true
        assert!(is_x86_64() || is_aarch64());

        // Should be mutually exclusive
        assert!(!(is_x86_64() && is_aarch64()));
    }

    #[test]
    fn test_qemu_arch() {
        // OCI/Go names normalize to QEMU spelling
        assert_eq!(qemu_arch("amd64"), "x86_64");
        assert_eq!(qemu_arch("arm64"), "aarch64");
        // QEMU/Rust names pass through
        assert_eq!(qemu_arch("x86_64"), "x86_64");
        assert_eq!(qemu_arch("aarch64"), "aarch64");
        // Unknown values are returned unchanged
        assert_eq!(qemu_arch("riscv64"), "riscv64");
    }
}
