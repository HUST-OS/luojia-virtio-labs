use volatile_register::{RO, WO, RW};

/// MMIO Device Legacy Register Interface.
///
/// Ref: 4.2.4 Legacy interface
#[repr(C)]
pub struct VirtIoHeader {
    /// Magic value
    magic: RO<u32>,

    /// Device version number
    ///
    /// Legacy device returns value 0x1.
    version: RO<u32>,

    /// Virtio Subsystem Device ID
    device_id: RO<u32>,

    /// Virtio Subsystem Vendor ID
    vendor_id: RO<u32>,

    /// Flags representing features the device supports
    device_features: RO<u32>,

    /// Device (host) features word selection
    device_features_sel: WO<u32>,

    /// Reserved
    __r1: [u32; 2],

    /// Flags representing device features understood and activated by the driver
    driver_features: WO<u32>,

    /// Activated (guest) features word selection
    driver_features_sel: WO<u32>,

    /// Guest page size
    ///
    /// The driver writes the guest page size in bytes to the register during
    /// initialization, before any queues are used. This value should be a
    /// power of 2 and is used by the device to calculate the Guest address
    /// of the first queue page (see QueuePFN).
    guest_page_size: WO<u32>,

    /// Reserved
    __r2: u32,

    /// Virtual queue index
    ///
    /// Writing to this register selects the virtual queue that the following
    /// operations on the QueueNumMax, QueueNum, QueueAlign and QueuePFN
    /// registers apply to. The index number of the first queue is zero (0x0).
    queue_sel: WO<u32>,

    /// Maximum virtual queue size
    ///
    /// Reading from the register returns the maximum size of the queue the
    /// device is ready to process or zero (0x0) if the queue is not available.
    /// This applies to the queue selected by writing to QueueSel and is
    /// allowed only when QueuePFN is set to zero (0x0), so when the queue is
    /// not actively used.
    queue_num_max: RO<u32>,

    /// Virtual queue size
    ///
    /// Queue size is the number of elements in the queue. Writing to this
    /// register notifies the device what size of the queue the driver will use.
    /// This applies to the queue selected by writing to QueueSel.
    queue_num: WO<u32>,

    /// Used Ring alignment in the virtual queue
    ///
    /// Writing to this register notifies the device about alignment boundary
    /// of the Used Ring in bytes. This value should be a power of 2 and
    /// applies to the queue selected by writing to QueueSel.
    queue_align: WO<u32>,

    /// Guest physical page number of the virtual queue
    ///
    /// Writing to this register notifies the device about location of the
    /// virtual queue in the Guest’s physical address space. This value is
    /// the index number of a page starting with the queue Descriptor Table.
    /// Value zero (0x0) means physical address zero (0x00000000) and is illegal.
    /// When the driver stops using the queue it writes zero (0x0) to this
    /// register. Reading from this register returns the currently used page
    /// number of the queue, therefore a value other than zero (0x0) means that
    /// the queue is in use. Both read and write accesses apply to the queue
    /// selected by writing to QueueSel.
    queue_pfn: RW<u32>,

    /// new interface only
    queue_ready: RW<u32>,

    /// Reserved
    __r3: [u32; 2],

    /// Queue notifier
    queue_notify: WO<u32>,

    /// Reserved
    __r4: [u32; 3],

    /// Interrupt status
    interrupt_status: RO<u32>,

    /// Interrupt acknowledge
    interrupt_ack: WO<u32>,

    /// Reserved
    __r5: [u32; 2],

    /// Device status
    ///
    /// Reading from this register returns the current device status flags.
    /// Writing non-zero values to this register sets the status flags,
    /// indicating the OS/driver progress. Writing zero (0x0) to this register
    /// triggers a device reset. The device sets QueuePFN to zero (0x0) for
    /// all queues in the device. Also see 3.1 Device Initialization.
    status: RW<DeviceStatus>,

    /// Reserved
    __r6: [u32; 3],

    // new interface only since here
    queue_desc_low: WO<u32>,
    queue_desc_high: WO<u32>,

    /// Reserved
    __r7: [u32; 2],

    queue_avail_low: WO<u32>,
    queue_avail_high: WO<u32>,

    /// Reserved
    __r8: [u32; 2],

    queue_used_low: WO<u32>,
    queue_used_high: WO<u32>,

    /// Reserved
    __r9: [u32; 21],

    config_generation: RO<u32>,
}

impl VirtIoHeader {
    /// Verify a valid header.
    pub fn verify(&self) -> bool {
        self.magic.read() == 0x7472_6976 && self.version.read() == 1 && self.device_id.read() != 0
    }
}

bitflags::bitflags! {
    /// The device status field.
    struct DeviceStatus: u32 {
        /// Indicates that the guest OS has found the device and recognized it
        /// as a valid virtio device.
        const ACKNOWLEDGE = 1;

        /// Indicates that the guest OS knows how to drive the device.
        const DRIVER = 2;

        /// Indicates that something went wrong in the guest, and it has given
        /// up on the device. This could be an internal error, or the driver
        /// didn’t like the device for some reason, or even a fatal error
        /// during device operation.
        const FAILED = 128;

        /// Indicates that the driver has acknowledged all the features it
        /// understands, and feature negotiation is complete.
        const FEATURES_OK = 8;

        /// Indicates that the driver is set up and ready to drive the device.
        const DRIVER_OK = 4;

        /// Indicates that the device has experienced an error from which it
        /// can’t recover.
        const DEVICE_NEEDS_RESET = 64;
    }
}
