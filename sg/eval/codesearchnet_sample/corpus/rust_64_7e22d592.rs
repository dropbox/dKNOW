pub fn try_rotated_by(mut self, rotation: Quat) -> Result<Self, MeshAccessError> {
        self.try_rotate_by(rotation)?;
        Ok(self)
    }