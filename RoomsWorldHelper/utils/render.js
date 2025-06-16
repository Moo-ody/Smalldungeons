
const MCTessellator = Java.type("net.minecraft.client.renderer.Tessellator")./* getInstance */func_178181_a()
const DefaultVertexFormats = Java.type("net.minecraft.client.renderer.vertex.DefaultVertexFormats")
const WorldRenderer = MCTessellator./* getWorldRenderer */func_178180_c()

const lerpViewEntity = (pticks) => {
    if (!pticks) pticks = Tessellator.getPartialTicks()
    const entity = Client.getMinecraft()./* getRenderViewEntity */func_175606_aa()

    return [
        entity./* lastTickPosX */field_70142_S + (entity./* posX */field_70165_t - entity./* lastTickPosX */field_70142_S) * pticks,
        entity./* lastTickPosY */field_70137_T + (entity./* posY */field_70163_u - entity./* lastTickPosY */field_70137_T) * pticks,
        entity./* lastTickPosZ */field_70136_U + (entity./* posZ */field_70161_v - entity./* lastTickPosZ */field_70136_U) * pticks
    ]
}

export const renderOutlinedBox = (x0, y0, z0, x1, y1, z1, r, g, b, a, lineWidth, phase) => {
    const [ rx, ry, rz ] = lerpViewEntity(null)

    GlStateManager./* pushMatrix */func_179094_E()
    GlStateManager./* disableTexture2D */func_179090_x()
    // Technically not needed because it's renderWorld
    // GlStateManager.disableLighting()
    if (phase) {
        GlStateManager./* disableDepth */func_179097_i()
    }
    GlStateManager./* enableBlend */func_179147_l()
    GlStateManager./* tryBlendFuncSeparate */func_179120_a(770, 771, 1, 0)
    GlStateManager./* translate */func_179137_b(-rx, -ry, -rz)
    GlStateManager./* color */func_179131_c(r, g, b, 1)
    GlStateManager.func_179129_p(); // disableCullFace

    GL11.glLineWidth(lineWidth)
    
    WorldRenderer./* begin */func_181668_a(GL11.GL_LINE_STRIP, DefaultVertexFormats./* POSITION */field_181705_e)
    Tessellator.colorize(r, g, b, a)

    WorldRenderer./* pos */func_181662_b(x0, y0, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y0, z1)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y1, z1)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y1, z1)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y1, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y1, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y0, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y0, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y0, z1)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y0, z1)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y1, z1)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y1, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y1, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y0, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y0, z1)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y1, z1)./* endVertex */func_181675_d()

    MCTessellator./* draw */func_78381_a()
    
    GL11.glLineWidth(1)

    GlStateManager.func_179089_o(); // enableCull
    GlStateManager./* enableTexture2D */func_179098_w()
    if (phase) {
        GlStateManager./* enableDepth */func_179126_j()
    }
    GlStateManager./* disableBlend */func_179084_k()
    GlStateManager./* popMatrix */func_179121_F()
}

export const renderFilledBox = (x0, y0, z0, x1, y1, z1, r, g, b, a, phase) => {
    const [ rx, ry, rz ] = lerpViewEntity(null)

    GlStateManager./* pushMatrix */func_179094_E()
    GlStateManager./* disableTexture2D */func_179090_x()
    // Technically not needed because it's renderWorld
    // GlStateManager.disableLighting()
    if (phase) {
        GlStateManager./* disableDepth */func_179097_i()
    }
    GlStateManager./* enableBlend */func_179147_l()
    GlStateManager./* tryBlendFuncSeparate */func_179120_a(770, 771, 1, 0)
    GlStateManager./* translate */func_179137_b(-rx, -ry, -rz)
    GlStateManager./* color */func_179131_c(r, g, b, 1)
    GlStateManager.func_179129_p(); // disableCullFace

    WorldRenderer./* begin */func_181668_a(GL11.GL_QUADS, DefaultVertexFormats./* POSITION */field_181705_e)
    GlStateManager./* color */func_179131_c(r, g, b, a)

    WorldRenderer./* pos */func_181662_b(x1, y0, z1)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y0, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y0, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y0, z1)./* endVertex */func_181675_d()

    WorldRenderer./* pos */func_181662_b(x1, y1, z1)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y1, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y1, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y1, z1)./* endVertex */func_181675_d()

    WorldRenderer./* pos */func_181662_b(x0, y1, z1)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y1, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y0, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y0, z1)./* endVertex */func_181675_d()

    WorldRenderer./* pos */func_181662_b(x1, y1, z1)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y1, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y0, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y0, z1)./* endVertex */func_181675_d()

    WorldRenderer./* pos */func_181662_b(x1, y1, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y1, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y0, z0)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y0, z0)./* endVertex */func_181675_d()

    WorldRenderer./* pos */func_181662_b(x0, y1, z1)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y1, z1)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x1, y0, z1)./* endVertex */func_181675_d()
    WorldRenderer./* pos */func_181662_b(x0, y0, z1)./* endVertex */func_181675_d()

    MCTessellator./* draw */func_78381_a()

    GlStateManager.func_179089_o(); // enableCull
    GlStateManager./* enableTexture2D */func_179098_w()
    if (phase) {
        GlStateManager./* enableDepth */func_179126_j()
    }
    GlStateManager./* disableBlend */func_179084_k()
    GlStateManager./* popMatrix */func_179121_F()
}