local Error = require(script.Parent.Error)

local validTypes = {
    string = true,
    boolean = true,
    number = true,
    UDim = true,
    UDim2 = true,
    BrickColor = true,
    Color3 = true,
    Vector2 = true,
    Vector3 = true,
    NumberSequence = true,
    ColorSequence = true,
    NumberRange = true,
    Rect = true,
}

local function setAttribute(inst, name, value)
    local validName = typeof(name) == "string"
        and #name == math.clamp(#name, 1, 100)
        and name:match("[A-z0-9_]+")
        and name:sub(1, 3) ~= "RBX"

    if not validName then
        return false, Error.new(Error.Kind.InvalidAttributeName, {
            inst = inst,
            name = name,
            value = value,
        })
    end

    if value ~= nil then
        local valueType = typeof(value)

        if not validTypes[valueType] then
            return false, Error.new(Error.Kind.UnsupportedAttributeValue, {
                inst = inst,
                name = name,
                value = value,
            })
        end
    end

    -- TODO: Does this pcall ever fail?
    return pcall(function()
        inst:SetAttribute(name, value)
    end)
end

return setAttribute