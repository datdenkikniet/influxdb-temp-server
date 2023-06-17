const ctx = document.querySelector('#chart')
const load_time = document.querySelector('#load-time')
const points_loaded = document.querySelector('#points-loaded')
const timespan = document.querySelector('#timespan')
const error_message = document.querySelector("#error")

const chart = new Chart(ctx, {
    type: 'line',
    options: {
        fill: true,
        parsing: false,
        plugins: {
            decimation: {
                enabled: true,
            },
            zoom: {
                zoom: {
                    wheel: {
                        enabled: true,
                    },
                    pinch: {
                        enabled: true,
                    },
                },
                pan: {
                    enabled: true,
                },
            }
        },
        scales: {
            xAxis: {
                type: "time",
            },
            y: {
                title: {
                    display: true,
                    text: 'Temperature (C)'
                }
            }
        }
    },
    data: {
        datasets: [{
            label: "Loading data...",
            data: [],
        }],
    }
});

function report_error(message) {
    document.querySelectorAll('.hide-until-loaded').forEach((e) => {
        e.hidden = true;
    })

    error_message.textContent = message
    error_message.hidden = false
}

async function fetch_data(span) {
    const url = "/temp/range/" + span

    const start = Date.now();

    let body

    try {
        const data = await fetch(url)

        if (data.status != 200) {
            report_error(await data.text())
            return []
        }

        body = await data.json();

        const end = Date.now();

        load_time.textContent = end - start;
        points_loaded.textContent = body.length;

        document.querySelectorAll('.hide-until-loaded').forEach((e) => {
            e.hidden = false;
        })

        error_message.hidden = true

        return body
    } catch (error) {
        report_error(error)
        return []
    } finally {
        document.querySelectorAll('.visible-until-loaded').forEach((e) => {
            e.hidden = true;
        })
    }
}

async function updatechart() {
    const data = await fetch_data(timespan.value)
    chart.data.labels.pop()
    chart.data.datasets.pop()

    const dataset = data.map((d) => { return { x: d.time, y: d.value } })

    chart.data.datasets.push({
        label: "Temperature (C)",
        data: dataset,
    })


    chart.update()
    chart.resetZoom()
}

updatechart()

timespan.onchange = () => {
    updatechart()
}