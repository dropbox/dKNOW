func (sb *storageBackend) SetVolumeAttachmentInfo(hostTag names.Tag, volumeTag names.VolumeTag, info VolumeAttachmentInfo) (err error) {
	defer errors.DeferredAnnotatef(&err, "cannot set info for volume attachment %s:%s", volumeTag.Id(), hostTag.Id())
	v, err := sb.Volume(volumeTag)
	if err != nil {
		return errors.Trace(err)
	}
	// Ensure volume is provisioned before setting attachment info.
	// A volume cannot go from being provisioned to unprovisioned,
	// so there is no txn.Op for this below.
	if _, err := v.Info(); err != nil {
		return errors.Trace(err)
	}
	// Also ensure the machine is provisioned.
	if _, ok := hostTag.(names.MachineTag); ok {
		m, err := sb.machine(hostTag.Id())
		if err != nil {
			return errors.Trace(err)
		}
		if _, err := m.InstanceId(); err != nil {
			return errors.Trace(err)
		}
	}
	return sb.setVolumeAttachmentInfo(hostTag, volumeTag, info)
}