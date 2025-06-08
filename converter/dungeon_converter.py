
import sys
import json

# 0: Nothing
# 1: Spawn
# 2: Fairy
# 3: Blood
# 4: Puzzle
# 5: Trap
# 6: Yellow
# 7+: Incremental unique rooms

# Formatted as 36 two digit numbers with no separation, 01010203

room_type_map = {
    "entrance": 1,
    "fairy": 2,
    "blood": 3,
    "puzzle": 4,
    "trap": 5,
    "yellow": 6
}

def get_room_num(room_id, rooms_json, curr_int, id_map):
    for entry in rooms_json:
        if entry["roomID"] != room_id:
            continue

        room_type = entry["type"]

        if room_type in room_type_map:
            return room_type_map[room_type]
        
        if room_id in id_map:
            return id_map.get(room_id)

        id_map[room_id] = curr_int + 1
        return curr_int + 1
    
    return 0

def convert_dungeon(line, rooms_json):
    _, __, rooms, doors = line.split(";")

    final = ""
    curr_int = 7
    id_map = {}

    for i in range(36):
        room_id = int(rooms[i*3:i*3+3])

        # Unknown room, invalid
        if room_id == 998:
            return None
        
        num = get_room_num(room_id, rooms_json, curr_int, id_map)

        if num > curr_int:
            curr_int = num
        
        if num < 10:
            final += "0" + str(num)
        else:
            final += str(num)


    return final + doors

def main():

    with open("converter/rooms.json") as f:
        rooms_json = json.load(f)

    if len(sys.argv) < 2:
        print("Missing input file")
        quit()
    
    filename = sys.argv[1]

    final = []

    with open(filename) as f:
        lines = f.read().splitlines()

        for i, line in enumerate(lines):
            print(f"{i+1}/{len(lines)} ({len(final)}) Dungeons")

            converted = convert_dungeon(line, rooms_json)
            # print(converted)

            if converted is None:
                continue

            final.append(converted)

    with open("output_dungeons.txt", "w") as f:
        f.write("\n".join(final))

    print("Done!")
main()