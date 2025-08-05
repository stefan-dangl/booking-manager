// Buttons
const refreshBtn = document.getElementById('refreshBtn');
const deleteAllBtn = document.getElementById('deleteAllBtn');
const addBtn = document.getElementById('addBtn');

// Timeslots
const timeslotsDiv = document.getElementById('timeslots');
const selectedSlotDisplay = document.getElementById('selectedSlotDisplay');

// Admin - Logout Button
const adminButtonGroup = document.querySelector('.admin-button-group');
const adminBtn = document.getElementById('adminBtn');
const logoutBtn = document.createElement('button');
logoutBtn.id = 'logoutBtn';
logoutBtn.textContent = 'Log out';
logoutBtn.classList.add('hidden'); 
adminButtonGroup.appendChild(logoutBtn);

// Admin Access Modal
const passwordModal = document.getElementById('passwordModal');
const adminPassword = document.getElementById('adminPassword');
const submitPasswordBtn = document.getElementById('submitPasswordBtn');
const cancelPasswordBtn = document.getElementById('cancelPasswordBtn');

// Booking Modal
const bookingForm = document.getElementById('bookingForm');
const confirmBookingBtn = document.getElementById('confirmBookingBtn');
const cancelBookingBtn = document.getElementById('cancelBookingBtn');
const deleteTimeslotBtn = document.getElementById('deleteTimeslotBtn');

// Delete All Confirmation Modal
const deleteAllModal = document.getElementById('deleteAllModal');
const confirmDeleteAllBtn = document.getElementById('confirmDeleteAllBtn');
const cancelDeleteAllBtn = document.getElementById('cancelDeleteAllBtn');

// Timeslot Modal
const addTimeslotModal = document.getElementById('addTimeslotModal');
const addTimeslotForm = document.getElementById('addTimeslotForm');
const cancelAddTimeslotBtn = document.getElementById('cancelAddTimeslotBtn');

// Hide admin buttons initially
deleteAllBtn.style.display = 'none';
addBtn.style.display = 'none';
deleteTimeslotBtn.classList.add('hidden');
logoutBtn.classList.add('hidden');

// Input validation
const bookerNameRegex = /^[\p{L}0-9 .!?-@_]+$/u;
const notesRegex = /^[\p{L}0-9 .!?@_#%*\-()+=:~\n£€¥$¢]+$/u;
document.getElementById('name').addEventListener('input', validateBookerName);
document.getElementById('newNotes').addEventListener('input', validateNewNotes);

let selectedTimeslot = null;
let adminPasswordCache = null;
let eventSource = null;
document.addEventListener('DOMContentLoaded', () => {
    setupTimeslotUpdate();
});

function setupTimeslotUpdate() {
    if (eventSource) {
        eventSource.close();
    }

    eventSource = new EventSource('http://0.0.0.0:PORT/timeslots');

    eventSource.onmessage = (event) => {
        try {
            const slots = JSON.parse(event.data);
            displayTimeslots(slots);
        } catch (error) {
            console.error('Error parsing timeslot data:', error);
        }
    };

    eventSource.onerror = (error) => {
        console.error('Failed to setup timeslot update:', error);
        setTimeout(setupTimeslotUpdate, 5000);
    };
}

function displayTimeslots(slots) {
    if (slots.length === 0) {
        timeslotsDiv.innerHTML = '<div>No timeslots available</div>';
        return;
    }

    timeslotsDiv.innerHTML = slots.map(slot => {
        const slotDate = new Date(slot.datetime);
        const isAvailable = slot.available;
        const isOutdated = slotDate < new Date(); 
        const notes = slot.notes;
        const booker_name = slot.booker_name;

        const formatNotes = (text) => {
            if (!text) return '';
            const maxLength = 26;
            const lines = [];
            while (text.length > 0) {
                lines.push(text.substring(0, maxLength));
                text = text.substring(maxLength);
            }
            return lines.join('<br>');
        };

        return `
    <div class="timeslot ${isOutdated ? 'outdated' : (isAvailable ? 'available' : 'booked')}"
        data-datetime="${slot.datetime}"
        data-available="${isAvailable}"
        data-outdated="${isOutdated}"
        data-id="${slot.id || ''}">
        <strong>${slotDate.toLocaleDateString()}</strong>
        <div>${slotDate.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</div>
        <div>${formatNotes(notes)}</div>
        <div>${!isAvailable ? 'Booked by ' + booker_name : (isOutdated ? 'Expired' : 'Available')}</div>
    </div>
    `;
    }).join('');

    document.querySelectorAll('.timeslot').forEach(slot => {
        slot.addEventListener('click', () => {
            const isBooked = slot.classList.contains('booked');
            const isOutdated = slot.dataset.outdated === 'true';

            // Admin can select any timeslot
            if (adminPasswordCache) {
                if (slot.classList.contains('admin-selected')) {
                    slot.classList.remove('admin-selected');
                    selectedTimeslot = null;
                    bookingForm.classList.add('hidden');
                } else {
                    document.querySelectorAll('.timeslot').forEach(s => {
                        s.classList.remove('selected');
                        s.classList.remove('admin-selected');
                    });
                    slot.classList.add('admin-selected');
                    selectedTimeslot = slot.dataset;
                    selectedSlotDisplay.textContent = new Date(selectedTimeslot.datetime).toLocaleString();
                    bookingForm.classList.remove('hidden');
                    confirmBookingBtn.disabled = isBooked || isOutdated;
                }
            }
            // Regular user can only select available timeslots
            else {
                if (isBooked || isOutdated) {
                    return;
                }
                if (slot.classList.contains('selected')) {
                    slot.classList.remove('selected');
                    selectedTimeslot = null;
                    bookingForm.classList.add('hidden');
                } else {
                    document.querySelectorAll('.timeslot').forEach(s => {
                        s.classList.remove('selected');
                    });
                    slot.classList.add('selected');
                    selectedTimeslot = slot.dataset;
                    selectedSlotDisplay.textContent = new Date(selectedTimeslot.datetime).toLocaleString();
                    bookingForm.classList.remove('hidden');
                    confirmBookingBtn.disabled = isBooked || isOutdated;
                }
            }
        });
    });
}

function validateBookerName() {
    const input = document.getElementById('name');
    const errorElement = document.getElementById('nameError');
    const value = input.value;

    if (value.length > 20) {
        input.value = value.substring(0, 20);
        return;
    }

    if (value.length > 0 && !bookerNameRegex.test(value)) {
        input.classList.add('input-error');
        errorElement.textContent = 'Invalid characters. Only letters, numbers, spaces, and .!?-@_ are allowed.';
        errorElement.classList.remove('hidden');
        return false;
    }

    input.classList.remove('input-error');
    errorElement.classList.add('hidden');
    return true;
}

function validateNewNotes() {
    const input = document.getElementById('newNotes');
    const errorElement = document.getElementById('newNotesError');
    const value = input.value;

    if (value.length > 81) {
        input.value = value.substring(0, 81);
        return;
    }

    if (value.length > 0 && !notesRegex.test(value)) {
        input.classList.add('input-error');
        errorElement.textContent = 'Invalid characters. Only letters, numbers, spaces, and .!?@_#%*-()+=:~\n£€¥$¢ are allowed.';
        errorElement.classList.remove('hidden');
        return false;
    }

    input.classList.remove('input-error');
    errorElement.classList.add('hidden');
    return true;
}

refreshBtn.addEventListener('click', setupTimeslotUpdate);

adminBtn.addEventListener('click', () => {
    passwordModal.style.display = 'block';
    adminPassword.focus();
});

submitPasswordBtn.addEventListener('click', async () => {
    const password = adminPassword.value.trim();

    if (!password) {
        alert('Please enter a password');
        return;
    }

    try {
        const response = await fetch('http://0.0.0.0:PORT/admin_page', {
            method: 'GET',
            headers: {
                'Content-Type': 'application/json',
                'x-admin-password': password
            },
        });

        if (!response.ok) {
            throw new Error('Invalid password or access denied');
        }

        adminPasswordCache = password;  // Cache the password for future requests
        passwordModal.style.display = 'none';
        adminPassword.value = '';

        // Enable delete-all and logout button after log-in
        deleteAllBtn.style.display = 'block';
        addBtn.style.display = 'block';
        adminBtn.classList.add('hidden');
        deleteTimeslotBtn.classList.remove('hidden');
        logoutBtn.classList.remove('hidden');
    } catch (error) {
        console.error('Failed to submit admin password:', error);
        alert(`Submit password error: ${error.message}`);
    }
});

cancelPasswordBtn.addEventListener('click', () => {
    passwordModal.style.display = 'none';
    adminPassword.value = '';
});

logoutBtn.addEventListener('click', () => {
    adminPasswordCache = null;
    deleteAllBtn.style.display = 'none';
    addBtn.style.display = 'none';
    deleteTimeslotBtn.classList.add('hidden');
    logoutBtn.classList.add('hidden');
    adminBtn.classList.remove('hidden');
    bookingForm.classList.add('hidden');

    // Clear any selected timeslots
    document.querySelectorAll('.timeslot').forEach(s => {
        s.classList.remove('selected');
        s.classList.remove('admin-selected');
    });
    selectedTimeslot = null;
});

deleteAllBtn.addEventListener('click', () => {
    deleteAllModal.style.display = 'block';
});

confirmDeleteAllBtn.addEventListener('click', async () => {
    try {
        if (!adminPasswordCache) {
            throw new Error('No admin credentials available');
        }

        const response = await fetch('http://0.0.0.0:PORT/remove_all', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'x-admin-password': adminPasswordCache
            },
            body: JSON.stringify({}),
        });

        const result = await response.text();
        if (!response.ok) {
            throw new Error(result);
        }
    } catch (error) {
        console.error('Failed to delete all timeslots:', error);
        alert(`Deletion error: ${error.message}`);
    }

    deleteAllModal.style.display = 'none';
});

cancelDeleteAllBtn.addEventListener('click', () => {
    deleteAllModal.style.display = 'none';
});

addBtn.addEventListener('click', () => {
    // Set default date/time (now + 1 hour)
    const now = new Date();
    now.setHours(now.getHours());
    now.setMinutes(0);
    now.setSeconds(0);
    now.setMilliseconds(0);

    // Format as YYYY-MM-DD
    const dateStr = now.toISOString().split('T')[0];
    // Format as HH:MM in 24-hour format
    const timeStr = `${String(now.getHours() + 1).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;

    document.getElementById('newDate').value = dateStr;
    document.getElementById('newTime').value = timeStr;
    document.getElementById('newNotes').value = '';

    addTimeslotModal.style.display = 'block';
});

addTimeslotForm.addEventListener('submit', async (e) => {
    try {
        if (!adminPasswordCache) {
            throw new Error('No admin credentials available');
        }

        e.preventDefault();

        const date = document.getElementById('newDate').value;
        const time = document.getElementById('newTime').value;
        const notes = document.getElementById('newNotes').value;

        if (!date || !time || !notes) {
            alert('Please fill in all required fields');
            return;
        }

        if (!validateNewNotes()) {
            return;
        }

        const [year, month, day] = date.split('-');
        const [hours, minutes] = time.split(':');
        const localDate = new Date(year, month - 1, day, hours, minutes);

        // Format as DateTime<Local> string
        const timeZoneOffset = -localDate.getTimezoneOffset();
        const offsetHours = Math.floor(Math.abs(timeZoneOffset) / 60).toString().padStart(2, '0');
        const offsetMinutes = (Math.abs(timeZoneOffset) % 60).toString().padStart(2, '0');
        const offsetSign = timeZoneOffset >= 0 ? '+' : '-';
        const timeZoneString = `${offsetSign}${offsetHours}:${offsetMinutes}`;

        // Format with microseconds (using milliseconds padded to 6 digits)
        const microseconds = String(localDate.getMilliseconds()).padStart(3, '0') + '000';
        const datetime = `${year}-${month.padStart(2, '0')}-${day.padStart(2, '0')}T` +
            `${hours.padStart(2, '0')}:${minutes.padStart(2, '0')}:00.${microseconds}${timeZoneString}`;

        const response = await fetch('http://0.0.0.0:PORT/add', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'x-admin-password': adminPasswordCache
            },
            body: JSON.stringify({
                datetime: datetime,
                notes: notes
            }),
        });

        const result = await response.text();
        if (!response.ok) {
            throw new Error(result);
        }
    } catch (error) {
        console.error('Failed to add timeslot:', error);
        alert(`Error adding timeslot: ${error.message}`);
    }

    addTimeslotModal.style.display = 'none';
});

cancelAddTimeslotBtn.addEventListener('click', () => {
    addTimeslotModal.style.display = 'none';
});

// Close modal when clicking outside
window.addEventListener('click', (event) => {
    if (event.target === passwordModal) {
        passwordModal.style.display = 'none';
        adminPassword.value = '';
    }
    if (event.target === deleteAllModal) {
        deleteAllModal.style.display = 'none';
    }
});

deleteTimeslotBtn.addEventListener('click', async () => {
    if (!selectedTimeslot) return;
    try {
        if (!adminPasswordCache) {
            throw new Error('No admin credentials available');
        }

        const response = await fetch('http://0.0.0.0:PORT/remove', {
            method: 'DELETE',
            headers: {
                'Content-Type': 'application/json',
                'x-admin-password': adminPasswordCache
            },
            body: JSON.stringify({
                id: selectedTimeslot.id
            }),
        });

        const result = await response.text();
        if (!response.ok) {
            throw new Error(result);
        }
    } catch (error) {
        console.error('Failed to delete timeslot:', error);
        alert(`Deletion error: ${error.message}`);
    }

    bookingForm.classList.add('hidden');
    selectedTimeslot = null;
});

cancelBookingBtn.addEventListener('click', () => {
    bookingForm.classList.add('hidden');
    document.querySelectorAll('.timeslot').forEach(s => {
        s.classList.remove('selected');
        s.classList.remove('admin-selected');
    });
    selectedTimeslot = null;
});

bookingForm.addEventListener('submit', async (e) => {
    e.preventDefault();

    const name = document.getElementById('name').value;

    if (!validateBookerName()) {
        return;
    }

    if (!selectedTimeslot || !name) {
        alert('Please select a timeslot and fill in your name');
        return;
    }

    try {
        const response = await fetch('http://0.0.0.0:PORT/book', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                id: selectedTimeslot.id,
                client_name: name,
            }),
        });

        const result = await response.text();
        if (!response.ok) {
            throw new Error(result);
        }
    } catch (error) {
        console.error("Failed to book timeslot:", error);
        alert(`Booking error: ${error.message}`);
    }

    bookingForm.reset();
    bookingForm.classList.add('hidden');
    selectedTimeslot = null;
});