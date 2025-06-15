
final = []
for z in range(11):
    for x in range(11):
        if x % 2 == 1 and z % 2 == 1 or x % 2 == 0 and z % 2 == 0:
            continue

        new_x = f"DUNGEON_ORIGIN.0 + {15 + x * 16}"
        new_z = f"DUNGEON_ORIGIN.1 + {15 + z * 16}"

        final.append((new_x, new_z))

print(final)

# for (let dz = 0; dz < 11; dz++) {
#     for (let dx = 0; dx < 11; dx++) {
#         if (dx % 2 == 1 && dz % 2 == 1 || dx % 2 == 0 && dz % 2 == 0) {
#             continue
#         }

#         let x = DUNGEON_CORNER_X + (ROOM_SIZE >> 1) + dx * ((ROOM_SIZE + 1) >> 1)
#         let z = DUNGEON_CORNER_Z + (ROOM_SIZE >> 1) + dz * ((ROOM_SIZE + 1) >> 1)

#         coords.push([x, 69, z])
#     }
# }