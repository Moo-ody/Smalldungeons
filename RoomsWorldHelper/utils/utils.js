const raw_rooms_json = JSON.parse(FileLib.read("RoomsWorldHelper", "data/new_rooms.json"))
export const room_info = []

let current_room_data = null

const roomDimensions = {
    "1x1": [31, 31],
    "1x2": [31+32, 31],
    "1x3": [31+32*2, 31],
    "1x4": [31+32*3, 31],
    "2x2": [31+32, 31+32],
    "L": [31+32, 31+32],
}

for (let i = 0; i < raw_rooms_json.length; i++) {
    let { ids, shape, name, type, room_id } = raw_rooms_json[i]
    let [width, length] = roomDimensions[shape]

    let ids_ints = ids.map(a => a.split(",").map(a => parseInt(a)))

    for (let j = 0; j < raw_rooms_json[i].ids.length; j++) {
        let id = raw_rooms_json[i].ids[j]
        let [x, z] = ids_ints[j]

        let new_data = {
            id,
            room_id, // IllegalMap room ID as it's used for dungeon layout
            x,
            z,
            name,
            type,
            shape,
            width,
            length
        }

        if ("doors" in raw_rooms_json[i]) {
            new_data.doors = raw_rooms_json[i].doors
        }
        
        room_info.push(new_data)
    }
}

export const get_curr_room = () => {
    return current_room_data
}


const isInRoomBounds = (x0, z0, room_data) => {
    const { x, z, width, length } = room_data

    if (x <= x0 && x + width >= x0 && z <= z0 && z + length >= z0) {
        return true
    }

    return false
}

const find_room = () => {
    const x = Player.getX()
    const z = Player.getZ()

    const curr_data = get_curr_room()

    if (curr_data && isInRoomBounds(x, z, curr_data)) {
        return curr_data
    }

    for (let room of room_info) {
        if (isInRoomBounds(x, z, room)) {
            if (!room.height) {
                return import_room_from_file(room.id)
            }

            return room
        }
    }

    return null
}

/**
 * @callback RoomChangeFunction
 * @param new_data
 * @param old_data
*/
const room_change_funcs = []

/**
 * 
 * @param {RoomChangeFunction} func 
 */
export const on_room_change = (func) => {
    room_change_funcs.push(func)
}

register("tick", () => {
    if (Server.getIP() !== "localhost") {
        current_room_data = null
        return
    }
    const new_data = find_room()
    const old_data = current_room_data
    const changed = old_data !== new_data

    current_room_data = new_data

    if (changed) {
        for (let i = 0; i < room_change_funcs.length; i++) {
            room_change_funcs[i](new_data, old_data)
        }
    }
})

// [dx, dz, width_x, length_z] from the corner (0, 0) of the room, at y=69
const door_info = {
    "1x1": [
        [15, -1, 5, 7],
        [31, 15, 7, 5],
        [15, 31, 5, 7],
        [-1, 15, 7, 5],
    ],
    "1x2": [
        [15, -1, 5, 7],
        [47, -1, 5, 7],
        [63, 15, 7, 5],
        [47, 31, 5, 7],
        [15, 31, 5, 7],
        [-1, 15, 7, 5],
    ],
    "1x3": [
        [15, -1, 5, 7],
        [47, -1, 5, 7],
        [79, -1, 5, 7],
        [95, 15, 7, 5],
        [79, 31, 5, 7],
        [47, 31, 5, 7],
        [15, 31, 5, 7],
        [-1, 15, 7, 5],
    ],
    "1x4": [
        [15, -1, 5, 7],
        [47, -1, 5, 7],
        [79, -1, 5, 7],
        [111, -1, 5, 7],
        [127, 15, 7, 5],
        [111, 31, 5, 7],
        [79, 31, 5, 7],
        [47, 31, 5, 7],
        [15, 31, 5, 7],
        [-1, 15, 7, 5],
    ],
    "2x2": [
        [15, -1, 5, 7],
        [47, -1, 5, 7],
        [63, 15, 7, 5],
        [63, 47, 7, 5],
        [47, 63, 5, 7],
        [15, 63, 5, 7],
        [-1, 47, 7, 5],
        [-1, 15, 7, 5],
    ],
    "L": [
        [15, -1, 5, 7],
        [31, 15, 7, 5],
        [47, 31, 5, 7],
        [63, 47, 7, 5],
        [47, 63, 5, 7],
        [15, 63, 5, 7],
        [-1, 47, 7, 5],
        [-1, 15, 7, 5],
    ],
}

export const get_door_locations = (room_info) => {
    const { shape } = room_info

    if (!(shape in door_info)) {
        return []
    }

    const door_locations = door_info[shape]

    if (shape == "1x1") {
        if ("doors" in room_info) {
            return door_locations.filter((_, i) => room_info.doors[i] == "1")
        }

        return door_locations
    }

    return door_locations
}

export const get_lowest_block = (x, z) => {
    let y = -1

    while (y++ < 255) {
        let id = World.getBlockAt(x, y, z).type.getID()

        if (id == 0) {
            continue
        }

        return y
    }
    
    return null
}

export const get_highest_block = (x, z) => {
    for (let y = 255; y > 0; y--) {
        let id = World.getBlockAt(x, y, z)?.type?.getID()
        // Ignore gold blocks too because of Gold room with a random ass gold block on the roof sometimes.
        if (id == 0 || id == 4) {
            continue
        }
        
        return y
    }
    return null
}

const one_by_one_types = {
    "1000": "1x1_E",
    "0101": "1x1_I",
    "1011": "1x1_3",
    "0011": "1x1_L",
    "1111": "1x1_X",
}

export const get_1x1_variant = (room_info) => {
    if (!("doors" in room_info) || !(room_info.doors in one_by_one_types)) {
        return "1x1"
    }

    return one_by_one_types[room_info.doors]
}

const scan_room_blocks = (room_obj, do_all_blocks=true) => {
    const { x, z, width, length } = room_obj

    const bottom_pos = get_lowest_block(x, z)
    const top_pos = get_highest_block(x, z)

    let final = ""

    if (do_all_blocks) {
        for (let y = bottom_pos; y <= top_pos; y++) {
            for (let z_pos = z; z_pos < z + length; z_pos++) {
                for (let x_pos = x; x_pos < x + width; x_pos++) {
                    let block = World.getBlockAt(x_pos, y, z_pos)
    
                    let binary = (block.type.getID() << 4) | block.getMetadata()
                    let str = binary.toString(16)
    
                    final += "0".repeat(4 - str.length) + str
                }
            }
        }
        final = final.toUpperCase()
    }

    return {
        height: top_pos - bottom_pos,
        bottom: bottom_pos,
        block_data: final,
    }
}

export const make_default_data = (room_obj) => {
    const { height, bottom, block_data } = scan_room_blocks(room_obj, false)
    const { name, id, type, x, z, width, length, room_id } = room_obj

    let shape = room_obj.shape
    
    if (shape == "1x1") {
        shape = get_1x1_variant(room_obj)
    }

    return {
        name,
        id,
        room_id,
        x,
        z,
        shape,
        type,
        bottom,
        width,
        length,
        height,
        block_data
    }
}

export const export_room = (room_obj) => {
    const { room_id, name, id } = room_obj

    room_obj.block_data = scan_room_blocks(room_obj, true).block_data

    FileLib.write("RoomsWorldHelper", `output/${room_id},${name.replace(/ /g, "_").toLowerCase()},${id}.json`, JSON.stringify(room_obj), true)
}

/**
 * 
 * @param {String} room_id - The corner pos id
 */
export const import_room_from_file = (room_id) => {
    // ChatLib.chat(`room id: ${room_id}`)
    const room_entry_index = room_info.findIndex(a => a.id == room_id)

    if (room_entry_index == -1) {
        ChatLib.chat(`Could not find room!`)
        return null
    }

    const room_entry = room_info[room_entry_index]
    const id = room_entry.room_id
    const name = room_entry.name.replace(/ /g, "_").toLowerCase()

    const filename = `${id},${name},${room_id}.json`

    if (!FileLib.exists("RoomsWorldHelper", `output/${filename}`)) {
        ChatLib.chat(`File does not exist! creating default data.`)

        const data = make_default_data(room_entry)
        room_info[room_entry_index] = data

        return data
    }

    ChatLib.chat(`Imported successfully`)
    return JSON.parse(FileLib.read("RoomsWorldHelper", `output/${filename}`))
}