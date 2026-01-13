protected function acceptLabelEnd()
    {
        if (isset($this->label)) {
            if (isset($this->last_widget)) {
                $this->last_widget->setLabel($this->label->getText());
                unset($this->last_widget);
            } else {
                $this->left_over_labels[] = (clone $this->label);
            }
            unset($this->label);
        }
    }