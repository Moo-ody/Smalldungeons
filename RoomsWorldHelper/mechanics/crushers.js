import { renderOutlinedBox } from "../utils/render"
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

            if (dz > 0) {
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

register("command", (arg) => {
    if (!arg) {
        renderer.unregister()
        pos1 = null
        current_crusher = create_crusher_data()

        ChatLib.chat(`Click first point...`)
        setter.register()
        return
    }

    if (arg == "save") {
        const curr_room = get_curr_room()
        if (!curr_room) {
            ChatLib.chat(`No room!`)
            return
        }

        if (Object.values(current_crusher).includes(null)) {
            ChatLib.chat(`Null values! Set these before saving.\n${JSON.stringify(current_crusher)}`)
            return
        }


    }
}).setName("crusher")

const renderer = register("renderWorld", () => {
    if (pos1) {
        const [x, y, z] = pos1
        renderOutlinedBox(x, y, z, x+1, y+1, z+1, 1, 1, 0, 1, 1, true)
    }
    
    if (current_crusher.width && current_crusher.height) {
        let [x0, y0, z0] = current_crusher.position
        let { width, height, max_length } = current_crusher
        let dx = 1
        let dz = 1

        if (current_crusher.direction == 1 || current_crusher.direction == 3) {
            dz = width
        }
        else {
            dx = width
        }

        renderOutlinedBox(x0, y0, z0, x0 + dx, y0 + height, z0 + dz, 0, 1, 0, 1, 1, true)

        if (max_length) {
            let dx = 0
            let dz = 0
            if (current_crusher.direction == 0) {
                dx = width
                dz = -max_length
            }
            else if (current_crusher.direction == 1) {
                dx = max_length
                dz = width
            }
            else if (current_crusher.direction == 2) {
                dx = width
                dz = max_length
            }
            else {
                dx = -max_length
                dz = width
            }

            renderOutlinedBox(x0, y0, z0, x0 + dx, y0 + height, z0 + dz, 1, 0, 0, 1, 1, true)
        }

    }
}).unregister()