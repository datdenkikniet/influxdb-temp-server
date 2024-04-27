const ctx = document.querySelector('#chart')
const load_time = document.querySelector('#load-time')
const points_loaded = document.querySelector('#points-loaded')
const timespan = document.querySelector('#timespan')
const error_message = document.querySelector("#error")
const reset = document.querySelector("#reset")
const start_date = document.querySelector("#start-date")
const end_date = document.querySelector("#end-date")
const password = document.querySelector("#password")

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
        datasets: [],
    }
});

function report_error(message) {
    document.querySelectorAll('.hide-until-loaded').forEach((e) => {
        e.hidden = true;
    })

    error_message.textContent = message
    error_message.hidden = false
}

function calculate_saturation_pressure(temperature_c) {
    // Using the Buck equation: 
    // P = 0.61121 * exp((18.678 - (T / 234.5)) * (T / (257.14 + T)))

    let t_1 = temperature_c / 234.5
    let t_2 = temperature_c / (257.14 + temperature_c)
    let pow1 = (18.678 - t_1) * t_2
    let exp = Math.exp(pow1)
    return 0.61121 * exp
}

function absolute_humidity(data) {
    let temp = data.temperature
    let hum = data.humidity

    let saturation_pressure = calculate_saturation_pressure(temp)
    let vapor_pressure = saturation_pressure * (hum / 100)

    let moles = vapor_pressure / (461.5 * temp)

    return { time: data.time, value: moles * 18.02 * 1000 }
}

function convert(data) {
    let temp = data.map(v => { return { value: v.temperature, time: v.time } })
    let humidity = data.map(v => { return { value: v.humidity, time: v.time } })
    let abs_hum = data.map(absolute_humidity);

    return {
        temp: temp,
        humid: humidity,
        abs_humid: abs_hum,
    }
}

async function fetch_range_data(range) {
    let data_range = await fetch_data("/data/range/" + range)

    return convert(data_range)
}

async function fetch_range_data_between(start_ms, stop_ms) {
    let data_range = await fetch_data("/data/from/" + start_ms + "/to/" + stop_ms)

    return convert(data_range)
}

async function fetch_data(url) {
    const start = Date.now();

    let body

    try {
        const data = await fetch(url, { headers: { authorization: "Bearer " + password.value } })

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
    const dataset = data.temp.map((d) => { return { x: d.time, y: d.value } })
    const dataset1 = data.humid.map((d) => { return { x: d.time, y: d.value } })
    const dataset2 = data.abs_humid.map(d => { return { x: d.time, y: d.value }})

    if (chart.data.datasets.length == 0) {
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
    } else {
        chart.data.datasets[0].data = dataset
        chart.data.datasets[1].data = dataset1
    }

    chart.update('none')
}

timespan.onchange = () => {
    fetch_range_data(timespan.value).then(update_chart).then(() => chart.resetZoom())
}

password.onchange = () => {
    timespan.onchange()
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
