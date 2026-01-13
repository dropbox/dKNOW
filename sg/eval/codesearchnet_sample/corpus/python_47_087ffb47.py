def get_zone_names(self):
        """
        Get the name of all zones
        """
        zone_names = []
        for zone in self.get_zones():
            zone_names.append(zone['name'])

        return zone_names