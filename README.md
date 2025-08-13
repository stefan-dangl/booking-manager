# booking-manager

A lightweight, privacy-focused booking system designed for small-scale events.


## Playground 

You can try out the Booking Manager [here](https://booking-manager-latest.onrender.com/). 

Note: This is a shared demo environment—anyone using this link can view and modify existing timeslots and bookings. To access admin controls, click the "Admin" button and enter the password: ***password***.


## Background

My friend is teacher at a high school. During their IT focused project week, they needed a simple way to manage workshop registrations across different devices. 

This project focuses on:
- Full control over data (self-hosted)
- Zero-friction access from any device
- Seamless updates
- Just enough features to solve the problem without complexity


## How to use

### Client

1) Access the System
    - Open the server’s address in any modern web browser.
2) Book a Timeslot
    - Browse available timeslots and reserve one by entering your name.
3) Real-Time Updates
    - Timeslots are synchronized across all connected devices.
    - If your connection drops, manually refresh using the "Refresh Timeslots" button or reload the page.
4) Visual Feedback
    - Booked or expired timeslots change color and become unavailable for selection.
<p align="center">
<img src="docs/images/client_view.png" alt="Client view" width="800"  />
  <figcaption style="font-style: italic; margin-top: 8px;">
    Client view (no admin rights)
  </figcaption>
</p>

### Admin
1) Authentication
    - Click the admin button and enter the password to unlock admin rights 
2) Admin rights
    - Add new timeslots
    - Delete selected timeslots
    - Delete all timeslots
3) Automatic Cleanup
    - Expired timeslots (older than 1 day) are removed automatically. No manual maintenance needed.
<p align="center">
<img src="docs/images/admin_view.png" alt="Admin view" width="800"  />
  <figcaption style="font-style: italic; margin-top: 8px;">
    Admin view (provides additional buttons)
  </figcaption>
</p>

## How to run

### Docker (recommended)

```Bash
$ docker-compose up
```

### Natively

1) Setup rust (>= v1.88)
2) Navigate to the rust project: 
    ``` Bash
    $ cd src
    ```
3) Execute the application: 
    ``` Bash
    $ cargo run
    ```
4) In case you want your timeslots to be persistent you need to provide a Postgres database:
    - Install Postgres and Diesel
    - Adapt the files **.env** and **src/diesel.toml** according to your system
    - Run migration in the project root to configure your database: 
    ``` Bash
    $ diesel migration run
    ``` 


### Configuration

You can configure the Booking Manager either by adapting the **.env** file or by adding command line arguments. For help enter: 
``` Bash
$ cargo run -- -h
```
    
- Following can be configured:
    - Website title
        - By default the title is "Timeslot Booking Manager". You can change it to whatever you like. E.g. "IT Project Week"
    - Password
        - When requesting Admin rights, the password specified here has to be entered
    - Database Url and password
        - In case you want to run the project in persistent mode, you can define the url and password of your database here. Alternatively, you can run the project without database.
    - Port
        - Defines on which port the project runs
