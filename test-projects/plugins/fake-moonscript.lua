print('[plugin(fake-moonscript)] loading')

-- This does not actually compile moonscript, it is just to test the hooks that would be used for a
-- real one.

local function compile(moonscript)
  return moonscript
end

return function(options)
    print('[plugin(fake-moonscript)] create')

    return {
        name = 'fake-moonscript',
        middleware = function(id)
            print(('[plugin(fake-moonscript)] middleware: %s'):format(id))
            if id:match('%.moon$') then
              print('[plugin(fake-moonscript)] matched')
              if id:match('%.server%.moon$') then
                return 'lua_server'
              elseif id:match('%.client%.moon$') then
                return 'lua_client'
              else
                return 'lua_module'
              end
            end
            print('[plugin(fake-moonscript)] skipping')
        end,
        load = function(id)
            print(('[plugin(fake-moonscript)] load: %s'):format(id))
            if id:match('%.moon$') then
              print('[plugin(fake-moonscript)] matched')
              local contents = rojo.readFileAsUtf8(id)
              return compile(contents)
            end
            print('[plugin(fake-moonscript)] skipping')
        end
    }
end
