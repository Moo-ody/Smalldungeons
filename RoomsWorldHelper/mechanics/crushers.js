import { renderFilledBox, renderOutlinedBox } from "../utils/render"
import { get_curr_room } from "../utils/utils"

const crusher_format = {
    position: null,
    direction: null,
    width: null,
    height: null,
    max_length: null,
    tick_per_block: null,
    pause_duration: null,
}


const create_crusher_data = () => {
    const final = {}
    Object.keys(crusher_format).forEach(k => {
        final[k] = null
    })
    
    return final
}

let current_crusher = null
let pos1 = null

const setter = register("playerInteract", (action, _, event) => {
    if (action.toString() !== "RIGHT_CLICK_BLOCK") {
        return
    }

    cancel(event)
    
    const la = Player.lookingAt()
    if (!la || !(la instanceof Block)) {
        return
    }
    
    const x = la.getX()
    const y = la.getY()
    const z = la.getZ()

    if (current_crusher.width && current_crusher.height) {
        if (current_crusher.direction == 1) {
            let dx = current_crusher.position[0] - x

            if (dx > 0) {
                current_crusher.direction = 3
            }

            current_crusher.max_length = Math.abs(dx)
        }
        else {
            let dz = current_crusher.position[2] - z

            if (dz < 0) {
                current_crusher.direction = 2
            }

            current_crusher.max_length = Math.abs(dz)
        }

        ChatLib.chat(JSON.stringify(current_crusher, null, 4))

        setter.unregister()
        return
    }

    if (!pos1) {
        pos1 = [x, y, z]
        renderer.register()
        ChatLib.chat(`Set second point...`)
        return
    }
    else {
        current_crusher.position = [
            Math.min(pos1[0], x),
            Math.min(pos1[1], y),
            Math.min(pos1[2], z),
        ]

        current_crusher.width = Math.max(Math.abs(pos1[0] - x), Math.abs(pos1[2] - z)) + 1
        current_crusher.height = Math.abs(pos1[1] - y) + 1

        let dx = Math.abs(x - current_crusher.position[0])
        let dz = Math.abs(z - current_crusher.position[2])

        if (dx == Math.min(dx, dz)) {
            current_crusher.direction = 1
        }
        else {
            current_crusher.direction = 0
        }

        ChatLib.chat(JSON.stringify(current_crusher, null, 4))
        pos1 = null

        ChatLib.chat(`Width and height set. Click another spot to set the length.`)
        return
    }
}).unregister()

register("command", (arg, arg2) => {
    const curr_room = get_curr_room()

    if (!arg) {
        pos1 = null
        current_crusher = create_crusher_data()

        ChatLib.chat(`Click first point...`)
        setter.register()
        return
    }

    if (arg == "save") {
        if (current_crusher == null) {
            ChatLib.chat(`No crusher set!`)
            return
        }
        
        if (!curr_room) {
            ChatLib.chat(`No room!`)
            return
        }

        let valid = true
        for (let entry of Object.entries(current_crusher)) {
            let [key, value] = entry

            if (value == null) {
                ChatLib.chat(`Key "${key}" is null.`)
                valid = false
            }
        }

        if (!valid) {
            ChatLib.chat(`Null values present! Set these before saving.`)
            return
        }

        if (!("crushers" in curr_room)) {
            curr_room.crushers = []
        }

        current_crusher.position = [
            current_crusher.position[0] - curr_room.x,
            current_crusher.position[1],
            current_crusher.position[2] - curr_room.z,
        ]
        curr_room.crushers.push(current_crusher)
        current_crusher = null

        ChatLib.chat(`Added crusher entry to room. Do /exportroom to save it.`)
        return
    }

    if (arg == "delete") {
        if (current_crusher !== null) {
            setter.unregister()
            current_crusher = null
            ChatLib.chat(`Reset current crusher.`)
            return
        }

        if (!("crushers" in curr_room)) {
            ChatLib.chat(`Nothing to delete!`)
            return
        }

        let [first_x, first_y, first_z] = curr_room.crushers[0].position
        let x0 = Player.getX() - curr_room.x
        let z0 = Player.getZ() - curr_room.z

        let closest_ind = 0
        let closestDistSq = (x0 - first_x)**2 + (Player.getY() - first_y)**2 + (z0 - first_z)**2

        ChatLib.chat(closestDistSq)
        for (let i = 1; i < curr_room.crushers.length; i++) {
            let [x, y, z] = curr_room.crushers[i].position

            let dist = (x0 - x)**2 + (Player.getY() - y)**2 + (z0 - z)**2
            ChatLib.chat(dist)

            if (dist > closestDistSq) {
                continue
            }

            ChatLib.chat(`Updated to ${i}`)

            closest_ind = i
            closestDistSq = dist
        }

        curr_room.crushers.splice(closest_ind, 1)
        ChatLib.chat(`Deleted crusher. Do /exportroom to save.`)
        return
    }

    if (arg == "ticks") {
        let ticks = parseInt(arg2)

        if (isNaN(ticks) || ticks < 0) {
            ChatLib.chat(`Invalid tick amount`)
            return
        }

        current_crusher.tick_per_block = ticks
        ChatLib.chat(`Set current crusher's ticks per block to ${ticks}`)
        return
    }

    if (arg == "pause") {
        let ticks = parseInt(arg2)

        if (isNaN(ticks) || ticks < 0) {
            ChatLib.chat(`Invalid tick amount`)
            return
        }

        current_crusher.pause_duration = ticks
        ChatLib.chat(`Set current crusher's pause duration to ${ticks}`)
        return
    }
}).setTabCompletions(args => {
    const firstArg = ["save", "delete", "ticks", "pause"]

    return firstArg
}).setName("crusher")

const renderCrusher = (room, crusher_entry, translate=true) => {
    let [x0, y0, z0] = crusher_entry.position

    if (translate) {
        x0 = crusher_entry.position[0] + room.x
        z0 = crusher_entry.position[2] + room.z
    }

    renderFilledBox(x0, y0, z0, x0+1, y0+1, z0+1, 0, 1, 1, 0.5, true)
    
    if (crusher_entry.width && crusher_entry.height) {
        let { width, height, max_length } = crusher_entry
        let dx = 1
        let dz = 1

        if (crusher_entry.direction == 1 || crusher_entry.direction == 3) {
            dz = width
        }
        else {
            dx = width
        }

        renderOutlinedBox(x0, y0, z0, x0 + dx, y0 + height, z0 + dz, 0, 1, 0, 1, 1, true)
        renderFilledBox(x0, y0, z0, x0 + dx, y0 + height, z0 + dz, 0, 1, 0, 0.11, true)

        if (max_length) {
            let dx = 0
            let dz = 0
            if (crusher_entry.direction == 0) {
                dx = width
                dz = -max_length
            }
            else if (crusher_entry.direction == 1) {
                dx = max_length
                dz = width
            }
            else if (crusher_entry.direction == 2) {
                dx = width
                dz = max_length
            }
            else {
                dx = -max_length
                dz = width
            }

            renderOutlinedBox(x0, y0, z0, x0 + dx, y0 + height, z0 + dz, 1, 0, 0, 1, 1, true)
            renderFilledBox(x0, y0, z0, x0 + dx, y0 + height, z0 + dz, 1, 0, 0, 0.11, true)

            const midX = x0 + dx / 2
            const midY = y0 + crusher_entry.height / 2
            const midZ = z0 + dz / 2

            Tessellator.drawString(`Crusher`, midX, midY + 0.25, midZ, Renderer.WHITE, true, 0.02, false)
            Tessellator.drawString(`Ticks Per Block: ${crusher_entry.tick_per_block}`, midX, midY, midZ, Renderer.WHITE, true, 0.02, false)
            Tessellator.drawString(`Pause Duration: ${crusher_entry.pause_duration}`, midX, midY - 0.25, midZ, Renderer.WHITE, true, 0.02, false)

        }
    }
}

const renderer = register("renderWorld", () => {
    if (pos1) {
        const [x, y, z] = pos1
        renderOutlinedBox(x, y, z, x+1, y+1, z+1, 1, 1, 0, 1, 1, true)
    }

    const room = get_curr_room()

    if (!room) {
        return
    }

    if (current_crusher !== null && current_crusher.position !== null) {
        renderCrusher(room, current_crusher, false)
    }

    if (room && "crushers" in room) {
        for (let i = 0; i < room.crushers.length; i++) {
            // Tessellator.drawString(i, room.crushers[i].position[0] + room.x, room.crushers[i].position[1] + 1.5, room.crushers[i].position[2] + room.z, Renderer.WHITE, true, 0.02, false)
            renderCrusher(room, room.crushers[i], true)
        }
    }
    
})