const ctx = document.querySelector('#chart')
const load_time = document.querySelector('#load-time')
const points_loaded = document.querySelector('#points-loaded')
const timespan = document.querySelector('#timespan')
const error_message = document.querySelector("#error")
const reset = document.querySelector("#reset")
const start_date = document.querySelector("#start-date")
const end_date = document.querySelector("#end-date")

const chart = new Chart(ctx, {
    type: 'line',
    options: {
        parsing: false,
        pointStyle: false,
        plugins: {
            tooltip: {
                intersect: false,
                mode: 'index',
            },
            zoom: {
                zoom: {
                    wheel: {
                        enabled: true,
                    },
                    pinch: {
                        enabled: true,
                    },
                    onZoomComplete: on_zoom_complete_check,
                },
                pan: {
                    enabled: true,
                    mode: 'xy',
                    onPanComplete: on_range_change,
                },
            }
        },
        scales: {
            xAxis: {
                type: "time",
                time: {
                    displayFormats: {
                        minute: "EEE HH:mm",
                        hour: "EEE HH:mm",
                        day: "EEE HH:mm",
                        week: "EEE HH:mm",
                    }
                }
            },
            y: {
                title: {
                    display: true,
                    text: 'Temperature (C)'
                },
                position: 'left',
            },
            y1: {
                display: true,
                position: 'right',
                title: {
                    display: true,
                    text: 'Humidity (%H)'
                },
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

async function fetch_range_data(range) {
    return { temp: await fetch_data("/temp/range/" + range), humid: await fetch_data("/humidity/range/" + range) }
}

async function fetch_range_data_between(start_ms, stop_ms) {
    return {
        temp: await fetch_data("/temp/from/" + start_ms + "/to/" + stop_ms),
        humid: await fetch_data("/humidity/from/" + start_ms + "/to/" + stop_ms)
    }
}

async function fetch_data(url) {
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

        error_message.hidden = true

        document.querySelectorAll('.hide-until-loaded').forEach((e) => {
            e.hidden = false;
        })

        if (body.length > 0) {
            start_date.textContent = new Date(body[0].time).toLocaleString()
            end_date.textContent = new Date(body[body.length - 1].time).toLocaleString()
        } else {
            start_date.hidden = true
            end_date.hidden = true
        }

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

function update_chart(data) {
    chart.data.datasets.pop()
    chart.data.datasets.pop()

    const dataset = data.temp.map((d) => { return { x: d.time, y: d.value } })
    const dataset1 = data.humid.map((d) => { return { x: d.time, y: d.value } })

    chart.data.datasets.push({
        label: "Temperature (C)",
        data: dataset,
        yAxisID: 'y',
    })

    chart.data.datasets.push({
        label: "Humidity (%H)",
        data: dataset1,
        yAxisID: 'y1',
    })


    chart.update('none')
}

timespan.onchange = () => {
    fetch_range_data(timespan.value).then(update_chart).then(() => chart.resetZoom())
}

reset.onclick = () => {
    timespan.onchange()
}

async function on_zoom_complete_check() {
    const axis = chart.scales.xAxis;
    const min = Math.round(axis.min)
    const max = Math.round(axis.max)

    update_chart(await fetch_range_data_between(min, max))
}

async function on_range_change() {
    const axis = chart.scales.xAxis;
    const min = Math.round(axis.min)
    const max = Math.round(axis.max)

    update_chart(await fetch_range_data_between(min, max))
}

// Load initial data
fetch_range_data(timespan.value).then(update_chart)