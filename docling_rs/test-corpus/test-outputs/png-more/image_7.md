## Docker Data Location

Where should Docker store its data? On the root volume, if this workspace is destroyed and recreated with the same name, Docker's data will be reset. If you want to persist Docker's data or if you need more space than the root volume has, you can make it use the home volume here.

- Root Volume
- Home Volume