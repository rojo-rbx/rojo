--[[
    These are place ids that will not have metadata saved for them,
    such as last sync address or time. This is because they are not unique
    so storing metadata for them does not make sense as these ids are reused.
--]]

return {
    ["0"] = true, -- Local file
    ["95206881"] = true, -- Baseplate
    ["6560363541"] = true, -- Classic Baseplate
    ["95206192"] = true, -- Flat Terrain
    ["13165709401"] = true, -- Modern City
    ["520390648"] = true, -- Village
    ["203810088"] = true, -- Castle
    ["366130569"] = true, -- Suburban
    ["215383192"] = true, -- Racing
    ["264719325"] = true, -- Pirate Island
    ["203812057"] = true, -- Obby
    ["379736082"] = true, -- Starting Place
    ["301530843"] = true, -- Line Runner
    ["92721754"] = true, -- Capture The Flag
    ["301529772"] = true, -- Team/FFA Arena
    ["203885589"] = true, -- Combat
    ["10275826693"] = true, -- Concert
    ["5353920686"] = true, -- Move It Simulator
    ["6936227200"] = true, -- Mansion Of Wonder
}
