pub const fn from_timestamp(timestamp: Timestamp, slot_duration: SlotDuration) -> Self {
		Slot(timestamp.as_millis() / slot_duration.as_millis())
	}