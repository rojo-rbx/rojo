print('[plugin] loading: load-as-stringvalue.lua')

local function tableToString(t)
    local s = ''
    if type(t) == 'table' then
        s = s .. '{ '
        for k, v in next, t do
            if type(k) == 'number' then
                s = s .. tableToString(v)
            else
                s = s .. k .. ' = ' .. tableToString(v)
            end
        end
        s = s .. ' }'
    elseif type(t) == 'string' then
        s = s .. '"' .. t .. '"'
    else
        s = s .. tostring(t)
    end
    return s
end

return function(options)
    print(('[plugin] create with: %s'):format(tableToString(options)))
    options.extensions = options.extensions or {}

    return {
        name = 'load-as-stringvalue',
        middleware = function(id)
            print(('[plugin] middleware: %s'):format(id))
            local idExt = id:match('%.(%w+)$')
            for _, ext in next, options.extensions do
                if ext == idExt then
                    print(('[plugin] matched: %s'):format(ext))
                    return 'json_model'
                end
            end
            print('[plugin] skipping')
        end,
        load = function(id, contents)
            print(('[plugin] load: %s'):format(id))
            local idExt = id:match('%.(%w+)$')
            for _, ext in next, options.extensions do
                if ext == idExt then
                    print(('[plugin] matched: %s'):format(ext))
                    local encoded = contents:gsub('\n', '\\n')
                    return ('{"ClassName": "StringValue", "Properties": { "Value": "%s" }}'):format(encoded)
                end
            end
            print('[plugin] skipping')
        end
    }
end
