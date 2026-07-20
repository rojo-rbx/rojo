local CollectionService = game:GetService("CollectionService")

local AutomationValue = require(script.Parent.Parent.AutomationValue)

local Inspect = {}

local ROOT_SERVICES = {
	Lighting = true,
	ReplicatedStorage = true,
	ServerStorage = true,
	ServerScriptService = true,
	StarterGui = true,
	StarterPlayer = true,
	StarterPack = true,
	SoundService = true,
	Teams = true,
	TextChatService = true,
}

local BASE_PART_PROPERTIES = {
	"Anchored",
	"CanCollide",
	"CanQuery",
	"CanTouch",
	"CastShadow",
	"CFrame",
	"Position",
	"Orientation",
	"Size",
	"Color",
	"Material",
	"Transparency",
	"Reflectance",
	"Massless",
	"CollisionGroup",
}
local GUI_OBJECT_PROPERTIES = {
	"Visible",
	"Position",
	"Size",
	"AnchorPoint",
	"Rotation",
	"ZIndex",
	"BackgroundColor3",
	"BackgroundTransparency",
}
local CAMERA_PROPERTIES = { "CFrame", "Focus", "FieldOfView", "CameraType", "CameraSubject", "ViewportSize" }

local function quoteSegment(name)
	if name:match("^[A-Za-z_][A-Za-z0-9_]*$") then
		return name
	end
	return '"' .. name:gsub("\\", "\\\\"):gsub('"', '\\"'):gsub("\n", "\\n"):gsub("\r", "\\r"):gsub("\t", "\\t") .. '"'
end

local function childPath(parentPath, child)
	return parentPath .. "." .. quoteSegment(child.Name)
end

local function instancePath(instance)
	if instance == game then
		return "game"
	elseif instance == workspace then
		return "Workspace"
	end
	local segments = {}
	local current = instance
	while current ~= nil and current ~= game do
		table.insert(segments, 1, quoteSegment(current.Name))
		current = current.Parent
	end
	return table.concat(segments, ".")
end

local function isInDataModel(instance)
	local ok, result = pcall(function()
		return instance == game or instance:IsDescendantOf(game)
	end)
	return ok and result
end

local function resolveTarget(segments)
	local rootName = segments[1]
	local current
	if rootName == "game" then
		current = game
	elseif rootName == "Workspace" then
		current = workspace
	elseif ROOT_SERVICES[rootName] then
		local ok, service = pcall(game.GetService, game, rootName)
		if not ok then
			return nil, string.format("Could not resolve service '%s': %s", rootName, tostring(service))
		end
		current = service
	else
		return nil, string.format("Unsupported inspect root '%s'", tostring(rootName))
	end

	for index = 2, #segments do
		local found = nil
		for _, child in current:GetChildren() do
			if child.Name == segments[index] then
				found = child
				break
			end
		end
		if found == nil then
			return nil, string.format("Inspect target segment '%s' was not found", segments[index])
		end
		current = found
	end
	return current
end

local function appendProperties(instance, names, output, references, path)
	for _, name in names do
		local readOk, value = pcall(function()
			return instance[name]
		end)
		if not readOk then
			output[name] = AutomationValue.diagnostic(value)
		else
			local valuePath = if typeof(value) == "Instance" then instancePath(value) else path
			local encoded, encodeError = AutomationValue.encode(value, references, valuePath)
			output[name] = encoded or AutomationValue.diagnostic(encodeError)
		end
	end
end

local function readProperties(instance, references, path)
	local properties = {}
	appendProperties(instance, { "Archivable", "Parent" }, properties, references, path)
	if instance:IsA("BasePart") then
		appendProperties(instance, BASE_PART_PROPERTIES, properties, references, path)
	end
	if instance:IsA("Model") then
		appendProperties(instance, { "WorldPivot", "PrimaryPart" }, properties, references, path)
		local ok, cframe, size = pcall(instance.GetBoundingBox, instance)
		if ok then
			local encoded, encodeError = AutomationValue.encode({ cframe = cframe, size = size }, references, path)
			properties.BoundingBox = encoded or AutomationValue.diagnostic(encodeError)
		else
			properties.BoundingBox = AutomationValue.diagnostic(cframe)
		end
	end
	if instance:IsA("ValueBase") then
		appendProperties(instance, { "Value" }, properties, references, path)
	end
	if instance:IsA("GuiObject") then
		appendProperties(instance, GUI_OBJECT_PROPERTIES, properties, references, path)
	end
	if instance:IsA("LayerCollector") then
		appendProperties(instance, { "Enabled" }, properties, references, path)
	end
	if instance:IsA("ScreenGui") then
		appendProperties(instance, { "DisplayOrder", "IgnoreGuiInset" }, properties, references, path)
	end
	if instance:IsA("Camera") then
		appendProperties(instance, CAMERA_PROPERTIES, properties, references, path)
	end
	return properties
end

local function readAttributes(instance, references, path)
	local output = {}
	local ok, attributes = pcall(instance.GetAttributes, instance)
	if not ok then
		output["<attributes>"] = AutomationValue.diagnostic(attributes)
		return output
	end
	local keys = {}
	for key in attributes do
		table.insert(keys, key)
	end
	table.sort(keys)
	for _, key in keys do
		local encoded, encodeError = AutomationValue.encode(attributes[key], references, path)
		output[key] = encoded or AutomationValue.diagnostic(encodeError)
	end
	return output
end

local function readTags(instance)
	local ok, tags = pcall(CollectionService.GetTags, CollectionService, instance)
	if not ok then
		return {}
	end
	table.sort(tags)
	return tags
end

function Inspect.run(request, context)
	if request.target == nil or request.target.kind ~= "path" or type(request.target.segments) ~= "table" then
		return nil, "Malformed inspect target"
	end
	local target, targetError = resolveTarget(request.target.segments)
	if target == nil then
		return nil, targetError
	end
	if not isInDataModel(target) then
		return nil, "Inspect target was destroyed before traversal"
	end

	local pathSegments = table.create(#request.target.segments)
	for index, segment in request.target.segments do
		pathSegments[index] = if index == 1 then segment else quoteSegment(segment)
	end
	local rootPath = table.concat(pathSegments, ".")
	local visited = 0
	local globallyTruncated = false
	local truncationReason = nil

	local function visit(instance, path, depth)
		if visited >= request.maxInstances then
			globallyTruncated = true
			truncationReason = "maxInstances"
			return nil
		end
		if not isInDataModel(instance) then
			return nil
		end
		visited += 1
		local reference, referenceError = context.references:reference(instance, path)
		if reference == nil then
			return nil, referenceError
		end
		local node = {
			reference = reference,
			name = instance.Name,
			className = instance.ClassName,
			path = path,
			properties = if request.includeProperties then readProperties(instance, context.references, path) else {},
			attributes = if request.includeAttributes then readAttributes(instance, context.references, path) else {},
			tags = if request.includeTags then readTags(instance) else {},
			children = {},
			truncated = false,
		}
		if depth >= request.depth then
			return node
		end
		local children = instance:GetChildren()
		local count = math.min(#children, request.maxChildren)
		if #children > count then
			node.truncated = true
			globallyTruncated = true
			truncationReason = truncationReason or "maxChildren"
		end
		for index = 1, count do
			local child = visit(children[index], childPath(path, children[index]), depth + 1)
			if child == nil then
				node.truncated = true
				break
			end
			table.insert(node.children, child)
		end
		return node
	end

	local root, traversalError = visit(target, rootPath, 0)
	if root == nil then
		return nil, traversalError or "Inspect target was destroyed during traversal"
	end
	return {
		root = root,
		visitedInstances = visited,
		truncated = globallyTruncated,
		truncationReason = truncationReason,
	}
end

Inspect._test = {
	quoteSegment = quoteSegment,
	resolveTarget = resolveTarget,
}

return Inspect
