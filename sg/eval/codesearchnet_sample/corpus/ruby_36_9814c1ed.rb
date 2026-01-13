def method_missing(method_id, *arguments)
        possible_formats = MakeExportable.exportable_formats.keys.map(&:to_s).join('|')
        if match = /^create_(#{possible_formats})_report$/.match(method_id.to_s)
          format = match.captures.first
          self.create_report(format, *arguments)
        elsif match = /^to_(#{possible_formats})_export$/.match(method_id.to_s)
          format = match.captures.first
          self.to_export(format, *arguments)
        else
          super
        end
      end