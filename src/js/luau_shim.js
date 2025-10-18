const LUAU_URL = 'https://github.com/luau-lang/luau/releases/download/696/Luau.Web.js';

let modulePromise = null;
let executeScript = null;

function loadModule() {
    if (modulePromise) {
        return modulePromise;
    }
    modulePromise = new Promise((resolve, reject) => {
        if (typeof window === 'undefined') {
            reject(new Error('Luau runtime is only available in a browser environment'));
            return;
        }

        const ready = (module) => {
            if (module.calledRun) {
                resolve(module);
            } else {
                const previous = module.onRuntimeInitialized;
                module.onRuntimeInitialized = () => {
                    if (typeof previous === 'function') {
                        try {
                            previous();
                        } catch (err) {
                            console.error('Luau initialization handler failed', err);
                        }
                    }
                    resolve(module);
                };
            }
        };

        if (window.LuauModule && window.LuauModule.ccall) {
            ready(window.LuauModule);
            return;
        }

        const module = window.LuauModule || {};
        module.print = module.print || ((...args) => console.log('[Luau]', ...args));
        module.printErr = module.printErr || ((...args) => console.error('[Luau err]', ...args));
        window.LuauModule = module;
        window.Module = module;

        const script = document.createElement('script');
        script.src = LUAU_URL;
        script.async = true;
        script.onerror = () => reject(new Error('Failed to load Luau Web runtime'));
        script.onload = () => ready(module);
        document.head.appendChild(script);
    });
    return modulePromise;
}

export async function executeLuau(source, chunkName) {
    const module = await loadModule();
    const logs = [];
    const previousPrint = module.print;
    const previousPrintErr = module.printErr;
    module.print = (...args) => {
        logs.push(args.map(String).join(' '));
    };
    module.printErr = (...args) => {
        console.error('[Luau]', ...args);
    };

    if (!executeScript) {
        executeScript = module.cwrap('executeScript', 'number', ['string', 'string']);
    }

    const status = executeScript(source, chunkName || 'script');
    module.print = previousPrint;
    module.printErr = previousPrintErr;

    if (status !== 0) {
        throw new Error(`Luau returned status ${status}`);
    }

    let resultPayload = null;
    for (const line of logs) {
        if (typeof line === 'string' && line.startsWith('__HOST_RESULT__:')) {
            resultPayload = line.substring('__HOST_RESULT__:'.length);
        } else if (line && line.length > 0) {
            console.log('[Luau]', line);
        }
    }

    if (!resultPayload) {
        resultPayload = '{"changes":[],"wait":0,"finished":true}';
    }

    return resultPayload;
}
