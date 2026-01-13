fn Draw(&self, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32) {
        self.send_render_command(RenderCommand::Draw {
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        })
    }