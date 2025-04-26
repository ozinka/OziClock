using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Shapes;

namespace Ozi.Clock;

public class LineData
{
    public double Y { get; set; }
    public double X2 { get; set; }
    public double Y2 { get; set; }
}

public partial class TimeChecker2 : Window
{
    private readonly MainWindow _fFmMain;

    public TimeChecker2(MainWindow fmMain)
    {
        InitializeComponent();
        _fFmMain = fmMain;

        Loaded += Window_Loaded;
    }

    private void GridSplitter_LayoutUpdated(object sender, EventArgs e)
    {
        if (IsMouseOver)
            _fFmMain.NewFmTimeChecker.slTimeChecker.Value =
                rwTop.Height.Value * _fFmMain.NewFmTimeChecker.slTimeChecker.Maximum / rwTop.MaxHeight;
    }

    private void Window_Loaded(object sender, RoutedEventArgs e)
    {
        var canvases = new List<Canvas> { cnvLDN, cnvNY };
        foreach (var canvas in canvases)
        {
            InitializeCanvases();
        }
    }

    private double GetRegionOffset(string region)
    {
        // Define the time zone information for the regions.
        var timeZoneOffsets = new Dictionary<string, string?>
        {
            { "NY", "Eastern Standard Time" }, // New York (EST)
            { "LDN", "GMT Standard Time" }, // London (GMT)
            { "Kyiv", "FLE Standard Time" }, // Kyiv (FLE)
            { "PUN", "India Standard Time" }, // Pune (IST)
            { "SGP", "Singapore Standard Time" }, // Singapore (SGT)
            { "TKO", "Tokyo Standard Time" } // Tokyo (JST)
        };

        // Get the time zone info based on the region
        if (timeZoneOffsets.TryGetValue(region, out var timeZoneId))
        {
            var timeZoneInfo = TimeZoneInfo.FindSystemTimeZoneById(timeZoneId!);

            // Get current time in Kyiv and the region's timezone
            var kyivNow = TimeZoneInfo.ConvertTime(DateTime.UtcNow, TimeZoneInfo.Utc,
                TimeZoneInfo.FindSystemTimeZoneById("FLE Standard Time"));
            var regionNow = TimeZoneInfo.ConvertTime(DateTime.UtcNow, TimeZoneInfo.Utc, timeZoneInfo);

            // Calculate the offset in hours
            var timeDifference = regionNow - kyivNow;

            // Return the total offset in hours (including fractional hours for half-hour differences)
            return timeDifference.TotalHours;
        }

        // Default to Kyiv's time if region is not found
        return 0;
    }

    private void CreateLines(Canvas canvas, string region)
    {
        var lineData = new List<(double y, double x2)>();
        double y = 10;
        double[] pattern = { 25, 15, 15, 20, 15, 15 };

        // Add line data
        for (var i = 0; i < 24; i++) // 24 hours
        {
            foreach (var x2 in pattern)
            {
                lineData.Add((y, x2));
                y += 3;
            }
        }

        // Add one additional line with width 25
        lineData.Add((y, 25));

        // Clear the canvas
        canvas.Children.Clear();

        // Draw the lines
        foreach (var (lineY, x2) in lineData)
        {
            // Left side line
            canvas.Children.Add(new Line
            {
                X1 = 0,
                Y1 = lineY,
                X2 = x2,
                Y2 = lineY,
                Stroke = Brushes.Black,
                StrokeThickness = 1,
                SnapsToDevicePixels = true
            });

            // Right side line (mirrored)
            canvas.Children.Add(new Line
            {
                X1 = 100,
                Y1 = lineY,
                X2 = 100 - x2,
                Y2 = lineY,
                Stroke = Brushes.Black,
                StrokeThickness = 1,
                SnapsToDevicePixels = true
            });
        }

        // Calculate time offset for the region dynamically
        var offset = GetRegionOffset(region);

        double labelYOffset = -2; // Starting position for the first label

        // Add time labels considering the timezone offset for the region
        for (var i = 0; i < 25; i++)
        {
            // raw “hour” plus offset (can be fractional, e.g. Pune +5.5)
            var raw = i + offset;

            // wrap into [0,24)
            raw %= 24;
            if (raw < 0) raw += 24;

            // split into whole hours + minutes
            var hours = (int)Math.Floor(raw);
            var minutes = (int)Math.Round((raw - hours) * 60);

            // format label
            string timeLabel;
            if (hours == 0 && minutes == 0 && i == 24)
            {
                // show “24” instead of “0”
                timeLabel = "24";
            }
            else if (minutes > 0)
            {
                // e.g. “4:30”
                timeLabel = $"{hours}:{minutes:D2}";
            }
            else
            {
                // whole hour
                timeLabel = hours.ToString();
            }

            // Create the label for the adjusted time
            var label = new Label
            {
                Content = timeLabel,
                FontWeight = FontWeights.Bold,
                HorizontalContentAlignment = HorizontalAlignment.Center,
                Width = 99
            };

            // Add the label to the canvas
            canvas.Children.Add(label);
            label.SetValue(Canvas.LeftProperty, 0.0);
            label.SetValue(Canvas.TopProperty, labelYOffset);


            // Update the Y-coordinate for the next label
            labelYOffset += 18;
        }
    }

    private void InitializeCanvases()
    {
        // List of regions and their respective canvases
        var regions = new Dictionary<string, Canvas>
        {
            { "NY", cnvNY },
            { "LDN", cnvLDN },
            { "Kyiv", cnvKyiv },
            { "PUN", cnvPUN },
            { "SGP", cnvSGP },
            { "TKO", cnvTKO }
        };

        // Loop through each region and canvas
        foreach (var region in regions)
        {
            CreateLines(region.Value, region.Key);
        }
    }
}