/// <reference types="../CTAutocomplete" />

import "./mechanics/crushers"

import { renderFilledBox, renderOutlinedBox } from "./utils/render"
import { export_room, get_curr_room, get_door_locations, on_room_change, room_info } from "./utils/utils"

register("command", (room_name, id_index) => {
    const fixed_lower = room_name.replace(/_/g, " ").toLowerCase()
    const possible_rooms = []

    for (let i = 0; i < room_info.length; i++) {
        if (room_info[i].name.toLowerCase() !== fixed_lower) {
            continue
        }

        possible_rooms.push(room_info[i])

    }

    if (possible_rooms.length == 0) {
        ChatLib.chat(`Room not found!`)
        return
    }

    const ind = (parseInt(id_index) % possible_rooms.length) || 0
    const { x, z } = possible_rooms[ind]
    
    ChatLib.chat(`Ids: ${possible_rooms.map(a => a.id).join(" | ")}`)


    ChatLib.command(`tp @p ${x} 100 ${z}`)
    return
}).setTabCompletions((args) => {
    const room_names = [...new Set(room_info.map(a => a.name.replace(/ /g, "_").toLowerCase()))]

    if (args.length == 0) {
        return room_names
    }

    return room_names.filter(a => a.toLowerCase().startsWith(args[0].toLowerCase()))
}).setName("tproom")


const overlay = register("renderOverlay", () => {
    const curr_room = get_curr_room()
    const { name, shape } = curr_room

    const x = Renderer.screen.getWidth() / 2
    const y = 20

    const lines = [
        `You are in:`,
        `${name}`,
        `${shape}`,
    ]

    const la = Player.lookingAt()

    if (la instanceof Block) {
        const x = la.getX()
        const z = la.getZ()

        const dx = x - curr_room.x
        const dz = z - curr_room.z

        lines.push(`dx: ${dx}`)
        lines.push(`dz: ${dz}`)

    }

    for (let i = 0; i < lines.length; i++) {
        Renderer.drawString(lines[i], x - Renderer.getStringWidth(lines[i]) / 2, y + i*9)
    }
}).unregister()

const worldRenderer = register("renderWorld", () => {
    const curr_data = get_curr_room()

    const door_locations = get_door_locations(curr_data)
    // const DOOR_WIDTH = 5
    const DOOR_HEIGHT = 5

    const { x, z } = curr_data

    for (let i = 0; i < door_locations.length; i++) {
        let [dx, dz, width, length] = door_locations[i]
        
        renderOutlinedBox(x + dx - width/2+0.5, 69, z + dz - length/2+0.5, x + dx + width/2+0.5, 69 + DOOR_HEIGHT, z + dz + length/2+0.5, 0, 1, 0, 1, 2, false)
        renderFilledBox(x + dx - width/2+0.5, 69, z + dz - length/2+0.5, x + dx + width/2+0.5, 69 + DOOR_HEIGHT, z + dz + length/2+0.5, 0, 1, 0, 0.2, false)
    }
}).unregister()

on_room_change((new_data, old_data) => {
    if (new_data !== null) {
        overlay.register()
        worldRenderer.register()
    }
    else {
        overlay.unregister()
        worldRenderer.unregister()
    }
})

register("command", () => {
    const curr_room = get_curr_room()

    if (!curr_room) {
        ChatLib.chat(`No room!`)
        return
    }

    const started = Date.now()
    export_room(curr_room)

    ChatLib.chat(`Export took ${Date.now() - started}ms`)

}).setName("exportroom")

const GameType = Java.type("net.minecraft.world.WorldSettings$GameType")

const isSingleplayer = () => Server.getIP() !== "127.0.0.1:4972" || Server.getIP() == "localhost"

register("command", (arg1, arg2, arg3) => {
    if (arg1 == "speedboots") {
        ChatLib.command('give @p leather_boots 1 0 {display:{color:0,Name:"Speedy Boots"},ench:[{lvl:127,id:2}],AttributeModifiers:[{AttributeName:"generic.movementSpeed",Amount:0.4,UUIDLeast:276246000,UUIDMost:99,Name:1728205246861}],Unbreakable:1,HideFlags:5}')
        return
    }

    if (arg1 == "flyspeed") {
        if (!isSingleplayer()) {
            return
        }
        let speed = parseFloat(arg2)
        Player.getPlayer().field_71075_bZ.func_75092_a(speed)
        ChatLib.chat(`Fly speed set to ${speed}`)
        return
    }

    if (arg1 == "sp") {
        if (!isSingleplayer()) {
            return
        }
        const controller = Client.getMinecraft().field_71442_b
        controller.func_78746_a(GameType.SPECTATOR)
        Player.getPlayer().func_71033_a(GameType.SPECTATOR)

        ChatLib.chat(Player.getPlayer().func_175149_v())
    }
}).setTabCompletions(args => {
    const firstArg = ["speedboots", "flyspeed", "noclip", "sp", "c"]

    if (args.length == 0) {
        return firstArg
    }

    return firstArg
}).setName("roomshelper").setAliases(["rh"])
