

import json

with open("data/bettermapRooms.json") as f:
    bm = json.load(f)

with open("data/rooms.json") as f:
    im = json.load(f)

not_found = []
new_data = []

blacklisted_ids = []

replacements = {
    "Dragon": "Miniboss",
    "Default": "Miniboss",
}

for imroom in im:
    name = imroom["name"]
    found = False

    for bmroom in bm:
        if bmroom["name"] != name and not (name in replacements and bmroom["name"] == replacements[name]):
            continue
        
        found = True
        this_room = {
            "name": name,
            "type": imroom["type"],
            "room_id": imroom["roomID"],
            "ids": [i for i in bmroom["id"] if i not in blacklisted_ids],
            "shape": imroom["shape"],
        }

        if "doors" in imroom:
            this_room["doors"] = imroom["doors"]

        new_data.append(this_room)

        break

    if not found:
        not_found.append(name)

with open("data/export_rooms.json", "w") as f:
    json.dump(new_data, f, indent=4)

print(", ".join(not_found))