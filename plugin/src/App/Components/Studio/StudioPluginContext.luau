local Rojo = script:FindFirstAncestor("Rojo")
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local StudioPluginContext = Roact.createContext(nil)

return StudioPluginContext
