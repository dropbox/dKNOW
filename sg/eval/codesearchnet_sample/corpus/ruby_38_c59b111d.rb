def sub_match?(time, accessor, values)

      return true if values.nil?

      value = time.send(accessor)

      if accessor == :day

        values.each do |v|
          return true if v == 'L' && (time + DAY_S).day == 1
          return true if v.to_i < 0 && (time + (1 - v) * DAY_S).day == 1
        end
      end

      if accessor == :hour

        return true if value == 0 && values.include?(24)
      end

      if accessor == :monthdays

        return true if (values & value).any?
      end

      values.include?(value)
    end