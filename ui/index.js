const { invoke } = window.__TAURI__.tauri;

let ports = [];

async function refreshPorts() {
    try {
        ports = await invoke('get_active_ports');
        renderPorts();
    } catch (error) {
        console.error('Error fetching ports:', error);
    }
}

function renderPorts() {
    const listElement = document.getElementById('port-list');
    
    if (ports.length === 0) {
        listElement.innerHTML = `
            <div class="empty-state">
                <span>No hay puertos activos</span>
            </div>
        `;
        return;
    }

    listElement.innerHTML = ports.map(port => `
        <div class="port-item" id="port-${port.port}">
            <div class="port-info">
                <span class="port-number">Port ${port.port}</span>
                <span class="process-name">
                    ${port.name} 
                    <span class="pid-tag">PID ${port.pid}</span>
                </span>
            </div>
            <button class="kill-btn" onclick="killPort(${port.pid}, ${port.port})">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
                </svg>
            </button>
        </div>
    `).join('');
}

async function killPort(pid, portNumber) {
    if (confirm(`¿Estás seguro de que quieres cerrar el puerto ${portNumber} (PID ${pid})?`)) {
        try {
            await invoke('kill_port_process', { pid });
            // Add a small delay for the system to update
            setTimeout(refreshPorts, 500);
        } catch (error) {
            alert(`Error: ${error}`);
        }
    }
}

async function killAll() {
    if (ports.length === 0) return;
    
    if (confirm(`¿Estás seguro de que quieres cerrar TODOS (${ports.length}) los procesos activos?`)) {
        try {
            const pids = ports.map(p => p.pid);
            await invoke('kill_all_ports', { pids });
            setTimeout(refreshPorts, 500);
        } catch (error) {
            alert(`Error: ${error}`);
        }
    }
}

document.getElementById('kill-all').addEventListener('click', killAll);

// Initial load
refreshPorts();

// Refresh every 5 seconds
setInterval(refreshPorts, 5000);

// Globalize for onclick handlers
window.killPort = killPort;
