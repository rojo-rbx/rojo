--[[
	A map from Route objects (given by the server) to Roblox instances (created
	by the plugin).
]]

local function hashRoute(route)
	return table.concat(route, "/")
end

local RouteMap = {}
RouteMap.__index = RouteMap

function RouteMap.new()
	local self = {
		_map = {},
		_reverseMap = {},
		_connectionsByRbx = {},
	}

	setmetatable(self, RouteMap)

	return self
end

function RouteMap:insert(route, rbx)
	local hashed = hashRoute(route)

	-- Make sure that each route and instance are only present in RouteMap once.
	self:removeByRoute(route)
	self:removeByRbx(rbx)

	self._map[hashed] = rbx
	self._reverseMap[rbx] = hashed
	self._connectionsByRbx[rbx] = rbx.AncestryChanged:Connect(function(_, parent)
		if parent == nil then
			self:removeByRbx(rbx)
		end
	end)
end

function RouteMap:get(route)
	return self._map[hashRoute(route)]
end

function RouteMap:removeByRoute(route)
	local hashedRoute = hashRoute(route)
	local rbx = self._map[hashedRoute]

	if rbx ~= nil then
		self:_removeInternal(rbx, hashedRoute)
	end
end

function RouteMap:removeByRbx(rbx)
	local hashedRoute = self._reverseMap[rbx]

	if hashedRoute ~= nil then
		self:_removeInternal(rbx, hashedRoute)
	end
end

--[[
	Correcly removes the given Roblox Instance/Route pair from the RouteMap.
]]
function RouteMap:_removeInternal(rbx, hashedRoute)
	self._map[hashedRoute] = nil
	self._reverseMap[rbx] = nil
	self._connectionsByRbx[rbx]:Disconnect()
	self._connectionsByRbx[rbx] = nil

	self:_removeRbxDescendants(rbx)
end

--[[
	Ensure that there are no descendants of the given Roblox Instance still
	present in the map, guaranteeing that it has been cleaned out.
]]
function RouteMap:_removeRbxDescendants(parentRbx)
	for rbx in pairs(self._reverseMap) do
		if rbx:IsDescendantOf(parentRbx) then
			self:removeByRbx(rbx)
		end
	end
end

--[[
	Remove all items from the map and disconnect all connections, cleaning up
	the RouteMap.
]]
function RouteMap:destruct()
	self._map = {}
	self._reverseMap = {}

	for _, connection in pairs(self._connectionsByRbx) do
		connection:Disconnect()
	end

	self._connectionsByRbx = {}
end

function RouteMap:visualize()
	-- Log all of our keys so that the visualization has a stable order.
	local keys = {}

	for key in pairs(self._map) do
		table.insert(keys, key)
	end

	table.sort(keys)

	local buffer = {}
	for _, key in ipairs(keys) do
		local visualized = ("- %s: %s"):format(
			key,
			self._map[key]:GetFullName()
		)
		table.insert(buffer, visualized)
	end

	return table.concat(buffer, "\n")
end

return RouteMap
