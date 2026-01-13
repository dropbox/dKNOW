def rolling_fillna!(direction=:forward)
      enum = direction == :forward ? index : index.reverse_each
      last_valid_value = 0
      enum.each do |idx|
        if valid_value?(self[idx])
          last_valid_value = self[idx]
        else
          self[idx] = last_valid_value
        end
      end
      self
    end