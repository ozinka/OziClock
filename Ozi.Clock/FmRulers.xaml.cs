using System;
using System.Collections.Generic;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Effects;
using System.Windows.Shapes;

namespace Ozi.Utilities;

public class LineData
{
    public double Y { get; set; }
    public double X2 { get; set; }
    public double Y2 { get; set; }
}

public class VerticalMagnifierEffect : ShaderEffect
{
    private static readonly PixelShader _shader = new PixelShader
    {
        // UriSource = new Uri("pack://application:,,,/YourAssemblyName;component/VerticalMagnifier.ps")
        UriSource = new Uri("pack://application:,,,/Shaders/VerticalMagnifier.ps")
    };

    public VerticalMagnifierEffect()
    {
        PixelShader = _shader;

        UpdateShaderValue(InputProperty);
        UpdateShaderValue(CenterYProperty);
        UpdateShaderValue(BandHeightProperty);
        UpdateShaderValue(MagnificationProperty);
    }

    public static readonly DependencyProperty InputProperty =
        ShaderEffect.RegisterPixelShaderSamplerProperty("Input", typeof(VerticalMagnifierEffect), 0);

    public static readonly DependencyProperty CenterYProperty =
        DependencyProperty.Register("CenterY", typeof(double), typeof(VerticalMagnifierEffect),
            new UIPropertyMetadata(0.5, PixelShaderConstantCallback(0)));

    public static readonly DependencyProperty BandHeightProperty =
        DependencyProperty.Register("BandHeight", typeof(double), typeof(VerticalMagnifierEffect),
            new UIPropertyMetadata(0.2, PixelShaderConstantCallback(1)));

    public static readonly DependencyProperty MagnificationProperty =
        DependencyProperty.Register("Magnification", typeof(double), typeof(VerticalMagnifierEffect),
            new UIPropertyMetadata(2.0, PixelShaderConstantCallback(2)));

    public double CenterY
    {
        get => (double)GetValue(CenterYProperty);
        set => SetValue(CenterYProperty, value);
    }

    public double BandHeight
    {
        get => (double)GetValue(BandHeightProperty);
        set => SetValue(BandHeightProperty, value);
    }

    public double Magnification
    {
        get => (double)GetValue(MagnificationProperty);
        set => SetValue(MagnificationProperty, value);
    }
}

public partial class FmRulers : Window
{
    private readonly FmMainWindow _fFmMain;
    private bool _isDraggingVertical = false;
    private Point _startPointVertical;
    private double _originalLeftWidth;

    // constructor
    public FmRulers(FmMainWindow fmMain)
    {
        InitializeComponent();
        _fFmMain = fmMain;

        Loaded += Window_Loaded;
    }

    private void VerticalSplitter_MouseLeftButtonDown(object sender, MouseButtonEventArgs e)
    {
        _isDraggingVertical = true;
        _startPointVertical = e.GetPosition(gdRulers);
        _originalLeftWidth = colTopLeft.Width.Value;
        Mouse.Capture((UIElement)sender);
    }

    private void VerticalSplitter_MouseMove(object sender, MouseEventArgs e)
    {
        if (!_isDraggingVertical) return;
        if (e.LeftButton != MouseButtonState.Pressed)
        {
            _isDraggingVertical = false;
            Mouse.Capture(null);
            return;
        }

        Point currentPosition = e.GetPosition(gdRulers);
        double delta = currentPosition.X - _startPointVertical.X;
        double newWidth = _originalLeftWidth + delta;

        newWidth = Math.Max(colTopLeft.MinWidth, Math.Min(newWidth, colTopLeft.MaxWidth));
        colTopLeft.Width = new GridLength(newWidth, GridUnitType.Pixel);
    }

    private bool _isDraggingHorizontal = false;
    private Point _startPointHorizontal;
    private double _originalTopHeight;

    private void HorizontalSplitter_MouseLeftButtonDown(object sender, MouseButtonEventArgs e)
    {
        _isDraggingHorizontal = true;
        _startPointHorizontal = e.GetPosition(gdRulers);
        _originalTopHeight = rwTop.Height.Value;
        Mouse.Capture((UIElement)sender);
    }

    private void HorizontalSplitter_MouseMove(object sender, MouseEventArgs e)
    {
        if (!_isDraggingHorizontal) return;

        if (e.LeftButton != MouseButtonState.Pressed)
        {
            _isDraggingHorizontal = false;
            Mouse.Capture(null);
            return;
        }

        Point currentPosition = e.GetPosition(gdRulers); // must be gdRullers, not sender
        double deltaY = currentPosition.Y - _startPointHorizontal.Y;

        double newHeight = _originalTopHeight + deltaY;
        newHeight = Math.Max(0, Math.Min(newHeight, rwTop.MaxHeight));
        rwTop.Height = new GridLength(newHeight, GridUnitType.Pixel);
    }


    private void GridSplitter_LayoutUpdated(object sender, EventArgs e)
    {
        var position = RulerGrid.TranslatePoint(new Point(0, 0), this);

        var brush = (VisualBrush)this.Resources["glRulersBrush"];
        var oldViewbox = brush.Viewbox;

        // Update Y offset based on Grid position
        brush.Viewbox = new Rect(oldViewbox.X, position.Y, oldViewbox.Width, oldViewbox.Height);

        if (IsMouseOver)
        {
            _fFmMain.fmSlider.slTimeChecker.Value =
                rwTop.Height.Value * _fFmMain.fmSlider.slTimeChecker.Maximum / rwTop.MaxHeight;
        }
    }

    private void Window_Loaded(object sender, RoutedEventArgs e)
    {
        InitializeRulers();
        _isInitialized = true;
    }

    public double GetRegionOffset(string timeZoneId)
    {
        var nowUtc = DateTime.UtcNow;

        var timeZoneInfo = TimeZoneInfo.FindSystemTimeZoneById(timeZoneId!);

        // Get current time in Kyiv and the region's timezone
        var mainNow = TimeZoneInfo.ConvertTime(nowUtc, TimeZoneInfo.Utc,
            TimeZoneInfo.FindSystemTimeZoneById(App.Settings.MainTimeZone));
        var regionNow = TimeZoneInfo.ConvertTime(nowUtc, TimeZoneInfo.Utc, timeZoneInfo);

        // Calculate the offset in hours
        var timeDifference = regionNow - mainNow;

        // Return the total offset in hours (including fractional hours for half-hour differences)
        return timeDifference.TotalHours;
    }

    private void CreateLinesAndLabels(Canvas canvas, string timeZoneId)
    {
        var lineData = new List<(double y, double x2)>();
        double y = 15;
        double[] pattern = { 25, 15, 15, 20, 15, 15 };

        canvas.Children.Clear();
        // Add LINES
        // Add line data
        for (var i = 0; i < 24; i++) // 24 hours
        {
            foreach (var x2 in pattern)
            {
                lineData.Add((y, x2));
                y += 3;
            }
        }

        // Add one LAST line with width 25
        lineData.Add((y, 25));

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

        // Add LABELS
        // Calculate time offset for the region dynamically
        var offset = GetRegionOffset(timeZoneId);

        double labelYOffset = 6; // Starting position for the first label

        // Add time labels considering the timezone offset for the region
        for (var i = 0; i < 25; i++)
        {
            // raw “hour” plus offset (can be fractional, e.g. Pune +5.5)
            var raw = i + offset;

            // wrap into [0,24)
            raw %= 24;
            if (raw < 0)
            {
                raw += 24;
            }

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
            var label = new TextBlock()
            {
                Text = timeLabel,
                FontWeight = FontWeights.Bold,
                TextAlignment = TextAlignment.Center,
                Width = 99,
                // Background = Brushes.White,
            };

            // Add the label to the canvas
            canvas.Children.Add(label);
            label.SetValue(Canvas.LeftProperty, 0.0);
            label.SetValue(Canvas.TopProperty, labelYOffset);

            // Update the Y-coordinate for the next label
            labelYOffset += 18;
        }
    }

    private bool _isInitialized = false;

    public void UpdateRulers()
    {
        // Update the rulers
        if (_isInitialized)
        {
            foreach (var clock in App.LstClock)
            {
                CreateLinesAndLabels((Canvas)clock.RulerGrid.Children[0], clock.TimeZoneId);
            }
            colTopLeft.Width = new GridLength(App.Settings.MainClockIndex * 100, GridUnitType.Pixel);
        }
        
    }

    public void InitializeRulers()
    {
        ColumnDefinition colTopLeft = gdRulers.ColumnDefinitions[0];

        if (!_isInitialized)
        {
            foreach (var clock in App.LstClock)
            {
                glRulers.Children.Add(clock.RulerGrid);
                CreateLinesAndLabels((Canvas)clock.RulerGrid.Children[0], clock.TimeZoneId);
            }

            colTopLeft.MaxWidth = (this.glRulers.Children.Count - 1) * 100;
            colTopLeft.Width = new GridLength(App.Settings.MainClockIndex * 100, GridUnitType.Pixel);
        }
        else
        {
            double max = (this.glRulers.Children.Count - 1) * 100;
            colTopLeft.MaxWidth = max;

            double currentWidth = colTopLeft.Width.Value;
            colTopLeft.Width = new GridLength(Math.Min(max, currentWidth), GridUnitType.Pixel);
        }
    }
}